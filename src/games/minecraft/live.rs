//! A running server: who is online (query, RCON, ping, console `list`, in that order, until
//! one gives a complete answer) and running Java commands (RCON when usable, else the console).
use super::{
    CONSOLE, CONSOLE_AND_READ, PROPERTIES, console,
    java::{USERCACHE, usercache_players},
    ping, properties, query, rcon,
};
use crate::{
    console::split_lines,
    context::{Actor, Context},
    model::{Method, MutationResult, Online, OnlineSource, Player},
    tunnel::{self, TunnelError},
    validate,
};
use axum::http::StatusCode;
use shared::response::ApiResponse;
use std::collections::HashMap;
use wings_api::tunnel::QueryTcpTunnel;

const JAVA_PORT: u16 = 25565;
const RCON_PORT: u16 = 25575;
const BEDROCK_PORT: u16 = 19132;

/// Java's `list` with UUIDs, unaffected by plugins replacing `list`; servers without it answer
/// the plain `list`.
const LIST_COMMANDS: [&str; 2] = ["minecraft:list uuids", "list"];

/// A port from `server.properties`.
fn port(content: &str, key: &str) -> Option<u16> {
    properties::get_u32(content, key)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|&port| port != 0)
}

/// How `server.properties` lets a Java server be reached.
#[derive(Debug, PartialEq, Eq)]
struct Settings {
    server_port: Option<u16>,
    /// The query port, when `enable-query` is on.
    query: Option<u16>,
    /// The RCON port and password, when `enable-rcon` is on with a password.
    rcon: Option<(u16, String)>,
}

impl Settings {
    fn read(content: Option<&str>) -> Self {
        let content = content.unwrap_or_default();
        let server_port = port(content, "server-port");
        let query = (properties::get_bool(content, "enable-query") == Some(true)).then(|| {
            port(content, "query.port")
                .or(server_port)
                .unwrap_or(JAVA_PORT)
        });
        let rcon = match properties::get(content, "rcon.password") {
            Some(password)
                if !password.is_empty()
                    && properties::get_bool(content, "enable-rcon") == Some(true) =>
            {
                Some((
                    port(content, "rcon.port").unwrap_or(RCON_PORT),
                    password.to_string(),
                ))
            }
            _ => None,
        };
        Self {
            server_port,
            query,
            rcon,
        }
    }
}

fn may(actor: &Actor<'_>, permissions: &[&str]) -> bool {
    permissions
        .iter()
        .all(|permission| actor.permissions.has_server_permission(permission).is_ok())
}

/// What the sources tried so far gave: the fullest incomplete answer and the last error.
#[derive(Default)]
struct Attempts {
    partial: Option<Online>,
    error: Option<ApiResponse>,
}

impl Attempts {
    /// `online` when it is complete; otherwise kept when it names more players than the
    /// answer kept so far.
    fn answer(&mut self, online: Online) -> Option<Online> {
        if online.complete {
            return Some(online);
        }
        if self
            .partial
            .as_ref()
            .is_none_or(|kept| online.players.len() > kept.players.len())
        {
            self.partial = Some(online);
        }
        None
    }

    fn failed(&mut self, error: ApiResponse) {
        self.error = Some(error);
    }

    /// The fullest incomplete answer, else the last error.
    fn finish(self) -> Result<Online, ApiResponse> {
        match (self.partial, self.error) {
            (Some(online), _) => Ok(online),
            (None, Some(error)) => Err(error),
            (None, None) => {
                Err(ApiResponse::error("no source answered")
                    .with_status(StatusCode::GATEWAY_TIMEOUT))
            }
        }
    }
}

/// An answer to `list` (from the console or RCON).
fn list_answer(answer: console::ListAnswer, source: OnlineSource) -> Online {
    let players: Vec<Player> = answer
        .players
        .into_iter()
        .map(|(name, id)| Player { name, id })
        .collect();
    Online {
        complete: players.len() >= answer.count as usize,
        count: answer.count,
        max: answer.max,
        players,
        source,
    }
}

/// The players online from the console's answer to the first of `commands` that gets one.
async fn console_list(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    commands: &[&str],
) -> Result<Online, ApiResponse> {
    for command in commands {
        if let Some(answer) = ctx.ask(actor, command, console::parse_list).await? {
            return Ok(list_answer(answer, OnlineSource::Console));
        }
    }
    Err(
        ApiResponse::error("the server did not answer the list command")
            .with_status(StatusCode::GATEWAY_TIMEOUT),
    )
}

/// Opens a tunnel to the RCON port and logs in (wrap it in [`tunnel::bounded`]).
async fn connect(
    ctx: &Context<'_>,
    port: u16,
    password: &str,
) -> Result<rcon::Session<QueryTcpTunnel>, TunnelError> {
    let stream = ctx.tcp(port).await?;
    rcon::Session::login(stream, password).await
}

/// Runs a command over RCON, logged like a console command.
async fn rcon_command(
    session: &mut rcon::Session<QueryTcpTunnel>,
    actor: &Actor<'_>,
    command: &str,
) -> Result<String, TunnelError> {
    let reply = session.command(command).await?;
    actor
        .activity_logger
        .log(
            "server:console.command",
            serde_json::json!({ "command": command, "via": "rcon" }),
        )
        .await;
    Ok(reply)
}

async fn rcon_list(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    port: u16,
    password: &str,
) -> Result<Online, TunnelError> {
    let mut session = connect(ctx, port, password).await?;
    for command in LIST_COMMANDS {
        let reply = rcon_command(&mut session, actor, command).await?;
        if let Some(answer) = console::parse_list(&split_lines(&reply)) {
            return Ok(list_answer(answer, OnlineSource::Rcon));
        }
    }
    Err(TunnelError::failed("unexpected answer to list"))
}

/// The players a full stat names, with the UUIDs `usercache.json` knows for their names.
fn query_answer(stat: query::FullStat, known: &[Player]) -> Online {
    let ids: HashMap<String, &str> = known
        .iter()
        .filter_map(|player| Some((player.name.to_ascii_lowercase(), player.id.as_deref()?)))
        .collect();
    let players: Vec<Player> = stat
        .names
        .into_iter()
        .map(|name| Player {
            id: ids.get(&name.to_ascii_lowercase()).map(|id| id.to_string()),
            name,
        })
        .collect();
    Online {
        complete: players.len() == stat.count as usize,
        count: stat.count,
        max: stat.max,
        players,
        source: OnlineSource::Query,
    }
}

/// Players online on a Java server: query, RCON, Server List Ping, then the console, skipping
/// sources that are off, forbidden to the actor or failing.
pub async fn java(ctx: &Context<'_>, actor: &Actor<'_>) -> Result<Online, ApiResponse> {
    let settings = Settings::read(ctx.read_text_lossy(PROPERTIES).await?.as_deref());
    let mut attempts = Attempts::default();

    if let Some(port) = settings.query {
        match query::full_stat(ctx, port).await {
            Ok(stat) => {
                let usercache = ctx
                    .read_list(USERCACHE)
                    .await?
                    .and_then(Result::ok)
                    .unwrap_or_default();
                let online = query_answer(stat, &usercache_players(&usercache));
                if let Some(online) = attempts.answer(online) {
                    return Ok(online);
                }
            }
            Err(err) => attempts.failed(err.about("query").response()),
        }
    }

    if let Some((port, password)) = &settings.rcon
        && may(actor, CONSOLE)
    {
        match tunnel::bounded(rcon_list(ctx, actor, *port, password)).await {
            Ok(online) => {
                if let Some(online) = attempts.answer(online) {
                    return Ok(online);
                }
            }
            Err(err) => attempts.failed(err.about("RCON").response()),
        }
    }

    let address = ctx.address();
    let host = address
        .as_ref()
        .map_or("localhost", |(host, _)| host.as_str());
    let port = settings
        .server_port
        .or(address.as_ref().map(|&(_, port)| port))
        .unwrap_or(JAVA_PORT);
    match ping::java(ctx, host, port).await {
        Ok(online) => {
            if let Some(online) = attempts.answer(online) {
                return Ok(online);
            }
        }
        Err(err) => attempts.failed(err.about("Server List Ping").response()),
    }

    if may(actor, CONSOLE_AND_READ) {
        match console_list(ctx, actor, &LIST_COMMANDS).await {
            Ok(online) => {
                if let Some(online) = attempts.answer(online) {
                    return Ok(online);
                }
            }
            Err(err) => attempts.failed(err),
        }
    }
    attempts.finish()
}

/// Players online on a Bedrock server: the RakNet ping, then the console when the ping did
/// not settle it (the console also names the players).
pub async fn bedrock(ctx: &Context<'_>, actor: &Actor<'_>) -> Result<Online, ApiResponse> {
    let port = ctx
        .read_text_lossy(PROPERTIES)
        .await?
        .and_then(|content| port(&content, "server-port"))
        .unwrap_or(BEDROCK_PORT);
    let mut attempts = Attempts::default();
    match ping::bedrock(ctx, port).await {
        Ok(online) => {
            if let Some(online) = attempts.answer(online) {
                return Ok(online);
            }
        }
        Err(err) => attempts.failed(err.about("RakNet ping").response()),
    }
    if may(actor, CONSOLE_AND_READ) {
        match console_list(ctx, actor, &["list"]).await {
            Ok(online) => {
                if let Some(online) = attempts.answer(online) {
                    return Ok(online);
                }
            }
            Err(err) => attempts.failed(err),
        }
    }
    attempts.finish()
}

/// A logged-in RCON session when RCON is usable: enabled with a password, the actor may use
/// the console, and the tunnel and login work.
async fn rcon_session(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
) -> Result<Option<rcon::Session<QueryTcpTunnel>>, ApiResponse> {
    let settings = Settings::read(ctx.read_text_lossy(PROPERTIES).await?.as_deref());
    let Some((port, password)) = settings.rcon else {
        return Ok(None);
    };
    if !may(actor, CONSOLE) {
        return Ok(None);
    }
    match tunnel::bounded(connect(ctx, port, &password)).await {
        Ok(session) => Ok(Some(session)),
        Err(err) => {
            tracing::warn!("RCON is not usable, sending commands to the console: {err:?}");
            Ok(None)
        }
    }
}

/// Runs Java commands, through RCON when usable (the replies become the message), else
/// through the console. Once a command went over RCON nothing falls back to the console, so
/// no command runs twice.
pub async fn run(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    commands: &[&str],
) -> Result<MutationResult, ApiResponse> {
    for command in commands {
        validate::command(command)?;
    }
    let message = match rcon_session(ctx, actor).await? {
        Some(mut session) => {
            let mut replies = Vec::new();
            for command in commands {
                let reply = rcon_command(&mut session, actor, command)
                    .await
                    .map_err(|err| err.about("RCON").response())?;
                let reply = reply.trim();
                if !reply.is_empty() {
                    replies.push(reply.to_string());
                }
            }
            (!replies.is_empty()).then(|| replies.join("\n"))
        }
        None => {
            ctx.commands(actor, commands).await?;
            None
        }
    };
    Ok(MutationResult {
        method: Method::Command,
        restart_required: false,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn online(names: &[&str], count: u32, complete: bool) -> Online {
        Online {
            count,
            max: 20,
            players: names
                .iter()
                .map(|name| Player {
                    name: name.to_string(),
                    id: None,
                })
                .collect(),
            source: OnlineSource::Ping,
            complete,
        }
    }

    #[test]
    fn reads_ports_and_switches() {
        assert_eq!(
            Settings::read(None),
            Settings {
                server_port: None,
                query: None,
                rcon: None,
            }
        );
        assert_eq!(
            Settings::read(Some(
                "server-port=25570\nenable-query=true\nenable-rcon=true\nrcon.password=\n"
            )),
            Settings {
                server_port: Some(25570),
                query: Some(25570),
                rcon: None,
            }
        );
        assert_eq!(
            Settings::read(Some(
                "enable-query=true\nquery.port=25580\nenable-rcon=true\nrcon.port=25590\nrcon.password=s3cret\n"
            )),
            Settings {
                server_port: None,
                query: Some(25580),
                rcon: Some((25590, "s3cret".into())),
            }
        );
        assert_eq!(
            Settings::read(Some(
                "enable-query=true\nenable-rcon=false\nrcon.password=x\n"
            )),
            Settings {
                server_port: None,
                query: Some(JAVA_PORT),
                rcon: None,
            }
        );
        assert_eq!(
            Settings::read(Some("enable-rcon=true\nrcon.password=x\nserver-port=0\n")),
            Settings {
                server_port: None,
                query: None,
                rcon: Some((RCON_PORT, "x".into())),
            }
        );
    }

    #[test]
    fn complete_answers_win_and_partial_ones_wait() {
        let mut attempts = Attempts::default();
        assert_eq!(attempts.answer(online(&["Alex"], 3, false)), None);
        assert_eq!(attempts.answer(online(&[], 3, false)), None);
        assert_eq!(
            attempts.answer(online(&["Alex", "Steve", "Notch"], 3, true)),
            Some(online(&["Alex", "Steve", "Notch"], 3, true))
        );
        attempts.failed(ApiResponse::error("console").with_status(StatusCode::GATEWAY_TIMEOUT));
        // the fullest partial answer beats the error of a later source
        assert_eq!(attempts.finish().ok(), Some(online(&["Alex"], 3, false)));

        let mut failed = Attempts::default();
        failed.failed(TunnelError::failed("tunnel").response());
        assert_eq!(
            failed.finish().err().map(|error| error.status),
            Some(StatusCode::BAD_GATEWAY)
        );
    }

    #[test]
    fn list_answers_are_complete_when_they_name_everyone() {
        let answer = |count, names: &[&str]| console::ListAnswer {
            count,
            max: 20,
            players: names.iter().map(|name| (name.to_string(), None)).collect(),
        };
        assert!(list_answer(answer(2, &["a", "b"]), OnlineSource::Rcon).complete);
        assert!(list_answer(answer(0, &[]), OnlineSource::Console).complete);
        assert!(!list_answer(answer(3, &["a"]), OnlineSource::Console).complete);
    }

    #[test]
    fn query_answers_take_known_uuids() {
        let known = [Player {
            name: "Notch".into(),
            id: Some("069a79f4-44e9-4726-a5be-fca90e38aaf5".into()),
        }];
        let online = query_answer(
            query::FullStat {
                count: 2,
                max: 20,
                names: vec!["notch".into(), "jeb_".into()],
            },
            &known,
        );
        assert_eq!(
            online.players,
            [
                Player {
                    name: "notch".into(),
                    id: Some("069a79f4-44e9-4726-a5be-fca90e38aaf5".into()),
                },
                Player {
                    name: "jeb_".into(),
                    id: None,
                },
            ]
        );
        assert!(online.complete);
        assert_eq!(online.source, OnlineSource::Query);
    }
}

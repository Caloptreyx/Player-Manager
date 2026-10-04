pub mod get {
    use crate::{
        context::{
            BEDROCK_PERMISSIONS, Context, JAVA_BANS, JAVA_IP_BANS, JAVA_OPS, JAVA_USERCACHE,
            JAVA_WHITELIST, Level, PROPERTIES, Player, ServerState, bedrock_known, known_name,
            usercache_players,
        },
        edition::Edition,
        lists::{self, Entry},
        properties, validate,
    };
    use serde::Serialize;
    use serde_json::Value;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct WhitelistEntry {
        name: String,
        id: Option<String>,
        /// Bedrock only.
        ignores_player_limit: Option<bool>,
    }

    #[derive(ToSchema, Serialize)]
    struct Operator {
        /// `None` for Bedrock XUIDs no known player has.
        name: Option<String>,
        id: Option<String>,
        level: Level,
        /// Java only.
        bypasses_player_limit: Option<bool>,
    }

    #[derive(ToSchema, Serialize)]
    struct Ban {
        name: String,
        id: Option<String>,
        reason: Option<String>,
        source: Option<String>,
        created: Option<String>,
        expires: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct IpBan {
        ip: String,
        reason: Option<String>,
        source: Option<String>,
        created: Option<String>,
        expires: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct FileError {
        file: String,
        message: String,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        edition: Option<Edition>,
        state: ServerState,
        online_mode: Option<bool>,
        whitelist_enabled: Option<bool>,
        max_players: Option<u32>,
        whitelist: Vec<WhitelistEntry>,
        operators: Vec<Operator>,
        bans: Vec<Ban>,
        ip_bans: Vec<IpBan>,
        known: Vec<Player>,
        errors: Vec<FileError>,
    }

    /// The entries of a list file read for display; parse failures become `errors`.
    fn entries(
        file: &str,
        result: Option<Result<Vec<Value>, String>>,
        errors: &mut Vec<FileError>,
    ) -> Vec<Value> {
        match result {
            None => Vec::new(),
            Some(Ok(entries)) => entries,
            Some(Err(message)) => {
                errors.push(FileError {
                    file: file.to_string(),
                    message,
                });
                Vec::new()
            }
        }
    }

    fn string(entry: &Entry, key: &str) -> Option<String> {
        lists::string(entry, key).map(str::to_string)
    }

    fn java_id(entry: &Entry) -> Option<String> {
        lists::string(entry, "uuid").and_then(validate::java_id)
    }

    fn java_ban(entry: &Entry) -> Ban {
        Ban {
            name: string(entry, "name").unwrap_or_default(),
            id: java_id(entry),
            reason: string(entry, "reason"),
            source: string(entry, "source"),
            created: string(entry, "created"),
            expires: string(entry, "expires"),
        }
    }

    fn java_ip_ban(entry: &Entry) -> Option<IpBan> {
        Some(IpBan {
            ip: string(entry, "ip")?,
            reason: string(entry, "reason"),
            source: string(entry, "source"),
            created: string(entry, "created"),
            expires: string(entry, "expires"),
        })
    }

    fn java_operator(entry: &Entry) -> Operator {
        let level = entry
            .get("level")
            .and_then(Value::as_u64)
            .filter(|level| (1..=4).contains(level))
            .unwrap_or(4) as u8;
        Operator {
            name: string(entry, "name"),
            id: java_id(entry),
            level: Level::Number(level),
            bypasses_player_limit: Some(
                entry
                    .get("bypassesPlayerLimit")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        }
    }

    /// Players, lists and settings of the server.
    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = PAYLOAD_TOO_LARGE, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("files.read-content")?;

        let ctx = Context::load(&state, &server).await?;
        let mut response = Response {
            edition: ctx.edition,
            state: ctx.state,
            online_mode: None,
            whitelist_enabled: None,
            max_players: None,
            whitelist: Vec::new(),
            operators: Vec::new(),
            bans: Vec::new(),
            ip_bans: Vec::new(),
            known: Vec::new(),
            errors: Vec::new(),
        };
        let errors = &mut response.errors;

        match ctx.edition {
            None => {}
            Some(Edition::Java) => {
                let (content, whitelist, ops, bans, ip_bans, usercache) = tokio::try_join!(
                    ctx.read_text_lossy(PROPERTIES),
                    ctx.read_list(JAVA_WHITELIST),
                    ctx.read_list(JAVA_OPS),
                    ctx.read_list(JAVA_BANS),
                    ctx.read_list(JAVA_IP_BANS),
                    ctx.read_list(JAVA_USERCACHE),
                )?;
                if let Some(content) = content {
                    response.online_mode = properties::get_bool(&content, "online-mode");
                    response.whitelist_enabled = properties::get_bool(&content, "white-list");
                    response.max_players = properties::get_u32(&content, "max-players");
                }
                response.whitelist = lists::objects(&entries(JAVA_WHITELIST, whitelist, errors))
                    .filter_map(|entry| {
                        Some(WhitelistEntry {
                            name: string(entry, "name")?,
                            id: java_id(entry),
                            ignores_player_limit: None,
                        })
                    })
                    .collect();
                response.operators = lists::objects(&entries(JAVA_OPS, ops, errors))
                    .map(java_operator)
                    .collect();
                response.bans = lists::objects(&entries(JAVA_BANS, bans, errors))
                    .map(java_ban)
                    .collect();
                response.ip_bans = lists::objects(&entries(JAVA_IP_BANS, ip_bans, errors))
                    .filter_map(java_ip_ban)
                    .collect();
                response.known = usercache_players(&entries(JAVA_USERCACHE, usercache, errors));
            }
            Some(Edition::Bedrock) => {
                let allowlist_file = ctx.bedrock_allowlist();
                let (content, allowlist, permission_entries) = tokio::try_join!(
                    ctx.read_text_lossy(PROPERTIES),
                    ctx.read_list(allowlist_file),
                    ctx.read_list(BEDROCK_PERMISSIONS),
                )?;
                if let Some(content) = content {
                    response.online_mode = properties::get_bool(&content, "online-mode");
                    response.whitelist_enabled = properties::get_bool(&content, "allow-list")
                        .or_else(|| properties::get_bool(&content, "white-list"));
                    response.max_players = properties::get_u32(&content, "max-players");
                }
                let allowlist = entries(allowlist_file, allowlist, errors);
                let permission_entries = entries(BEDROCK_PERMISSIONS, permission_entries, errors);
                let known = bedrock_known(&allowlist, &ctx.log_lines(&permissions).await);

                response.whitelist = lists::objects(&allowlist)
                    .filter_map(|entry| {
                        Some(WhitelistEntry {
                            name: string(entry, "name")?,
                            id: lists::text(entry, "xuid")
                                .filter(|xuid| validate::bedrock_id(xuid)),
                            ignores_player_limit: Some(
                                entry
                                    .get("ignoresPlayerLimit")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            ),
                        })
                    })
                    .collect();
                response.operators = lists::objects(&permission_entries)
                    .map(|entry| {
                        let id = lists::text(entry, "xuid");
                        Operator {
                            name: id.as_deref().and_then(|xuid| known_name(&known, xuid)),
                            id,
                            level: Level::Name(
                                string(entry, "permission").unwrap_or_else(|| "member".to_string()),
                            ),
                            bypasses_player_limit: None,
                        }
                    })
                    .collect();
                response.known = known;
            }
        }

        ApiResponse::new_serialized(response).ok()
    }
}

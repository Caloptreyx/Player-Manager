use crate::lists::{self, Entry};
use std::net::IpAddr;

/// Whether the entry bans `ip`, however the address is written.
fn bans_ip(entry: &Entry, ip: IpAddr) -> bool {
    lists::string(entry, "ip").is_some_and(|found| {
        found
            .trim()
            .parse::<IpAddr>()
            .is_ok_and(|found| found == ip)
    })
}

pub mod add {
    use super::bans_ip;
    use crate::{
        context::{
            BAN_REASON, Context, JAVA_IP_BANS, Method, MutationResult, ban_created, java_only,
        },
        edition::Edition,
        lists, validate,
    };
    use serde::Deserialize;
    use serde_json::Value;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::ApiResponseResult,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        ip: String,
        #[serde(default)]
        reason: Option<String>,
    }

    /// Bans an IP address (Java only).
    #[utoipa::path(post, path = "/ip-bans", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        if ctx.require_edition()? == Edition::Bedrock {
            return Err(java_only());
        }
        let (_, method) = ctx.plan(&permissions)?;
        let ip = validate::ip(&data.ip)?;
        let reason = validate::reason(data.reason.as_deref())?;

        if method == Method::Command {
            let command = match &reason {
                Some(reason) => format!("ban-ip {ip} {reason}"),
                None => format!("ban-ip {ip}"),
            };
            ctx.command(&activity_logger, &command).await?;
            return MutationResult::respond(method, false);
        }

        let created = ban_created();
        let reason = reason.as_deref().unwrap_or(BAN_REASON);
        let mut entries = ctx.load_list(JAVA_IP_BANS).await?;
        lists::upsert(
            &mut entries,
            |entry| bans_ip(entry, ip),
            |entry| {
                entry.insert("ip".into(), Value::from(ip.to_string()));
                entry.insert("created".into(), Value::from(created.as_str()));
                entry.insert("source".into(), Value::from("Server"));
                entry.insert("expires".into(), Value::from("forever"));
                entry.insert("reason".into(), Value::from(reason));
            },
        );
        ctx.write_list(JAVA_IP_BANS, &entries, user.uuid, &activity_logger)
            .await?;

        MutationResult::respond(method, false)
    }
}

pub mod remove {
    use super::bans_ip;
    use crate::{
        context::{Context, JAVA_IP_BANS, Method, MutationResult, java_only},
        edition::Edition,
        lists, validate,
    };
    use axum::http::StatusCode;
    use serde::Deserialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        ip: String,
    }

    /// Pardons a banned IP address (Java only).
    #[utoipa::path(delete, path = "/ip-bans", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        if ctx.require_edition()? == Edition::Bedrock {
            return Err(java_only());
        }
        let (_, method) = ctx.plan(&permissions)?;
        let ip = validate::ip(&data.ip)?;

        if method == Method::Command {
            ctx.command(&activity_logger, &format!("pardon-ip {ip}"))
                .await?;
            return MutationResult::respond(method, false);
        }

        let mut entries = ctx.load_list(JAVA_IP_BANS).await?;
        if lists::remove(&mut entries, |entry| bans_ip(entry, ip)) == 0 {
            return Err(ApiResponse::error("IP ban not found").with_status(StatusCode::NOT_FOUND));
        }
        ctx.write_list(JAVA_IP_BANS, &entries, user.uuid, &activity_logger)
            .await?;

        MutationResult::respond(method, false)
    }
}

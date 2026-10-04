pub mod post {
    use crate::{
        context::{Context, Method, MutationResult},
        edition::Edition,
        validate,
    };
    use serde::Deserialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::ApiResponseResult,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        name: String,
        #[serde(default)]
        reason: Option<String>,
    }

    /// The `kick` command; Bedrock needs quotes around gamertags with spaces.
    fn command(edition: Edition, name: &str, reason: Option<&str>) -> String {
        let target = if edition == Edition::Bedrock && name.contains(' ') {
            format!("\"{name}\"")
        } else {
            name.to_string()
        };
        match reason {
            Some(reason) => format!("kick {target} {reason}"),
            None => format!("kick {target}"),
        }
    }

    /// Kicks an online player.
    #[utoipa::path(post, path = "/kick", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let edition = ctx.require_edition()?;
        ctx.require_running()?;
        permissions.has_server_permission("control.console")?;
        validate::name(edition, &data.name)?;
        let reason = validate::reason(data.reason.as_deref())?;

        ctx.command(
            &activity_logger,
            &command(edition, &data.name, reason.as_deref()),
        )
        .await?;

        MutationResult::respond(Method::Command, false)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn quotes_bedrock_names_with_spaces() {
            assert_eq!(
                command(Edition::Bedrock, "Some Guy", Some("afk")),
                "kick \"Some Guy\" afk"
            );
            assert_eq!(command(Edition::Bedrock, "Alex", None), "kick Alex");
            assert_eq!(
                command(Edition::Java, "Notch", Some("bye now")),
                "kick Notch bye now"
            );
        }
    }
}

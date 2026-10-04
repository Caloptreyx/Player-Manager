//! Player id lookups: Mojang profiles, GeyserMC gamertag → XUID, offline and Floodgate UUIDs.
use axum::http::StatusCode;
use serde::Deserialize;
use shared::response::ApiResponse;
use std::{sync::LazyLock, time::Duration};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("Calagopus-Player-Manager")
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build the player lookup http client")
});

fn upstream(service: &str, err: impl std::fmt::Display) -> ApiResponse {
    tracing::warn!("{service} lookup failed: {err}");
    ApiResponse::error(format!("{service} lookup failed")).with_status(StatusCode::BAD_GATEWAY)
}

/// `name` for a URL path segment (names are validated: letters, digits, `_` and spaces).
fn path_segment(name: &str) -> String {
    name.replace(' ', "%20")
}

/// The Mojang profile of a Java player name: `(dashed uuid, canonical name)`, `None` when no
/// account has that name.
pub async fn mojang_profile(name: &str) -> Result<Option<(String, String)>, ApiResponse> {
    #[derive(Deserialize)]
    struct Profile {
        id: String,
        name: String,
    }

    let response = CLIENT
        .get(format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            path_segment(name)
        ))
        .send()
        .await
        .map_err(|err| upstream("Mojang", err))?;
    match response.status() {
        reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::NOT_FOUND => return Ok(None),
        status if !status.is_success() => return Err(upstream("Mojang", status)),
        _ => {}
    }
    let profile: Profile = response
        .json()
        .await
        .map_err(|err| upstream("Mojang", err))?;
    let id = crate::validate::java_id(&profile.id)
        .ok_or_else(|| upstream("Mojang", format_args!("invalid id {:?}", profile.id)))?;
    Ok(Some((id, profile.name)))
}

/// The XUID of an Xbox gamertag from GeyserMC's cache; `None` when it is unknown.
pub async fn geyser_xuid(gamertag: &str) -> Result<Option<String>, ApiResponse> {
    let response = CLIENT
        .get(format!(
            "https://api.geysermc.org/v2/xbox/xuid/{}",
            path_segment(gamertag)
        ))
        .send()
        .await
        .map_err(|err| upstream("GeyserMC", err))?;
    match response.status() {
        reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::NOT_FOUND => return Ok(None),
        status if !status.is_success() => return Err(upstream("GeyserMC", status)),
        _ => {}
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|err| upstream("GeyserMC", err))?;
    let xuid = match body.get("xuid") {
        Some(serde_json::Value::Number(xuid)) => xuid.as_u64().map(|xuid| xuid.to_string()),
        Some(serde_json::Value::String(xuid)) => Some(xuid.clone()),
        _ => None,
    };
    Ok(xuid.filter(|xuid| crate::validate::bedrock_id(xuid)))
}

/// The UUID an offline-mode Java server gives `name`: Java's
/// `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`, an MD5 hash with the version 3 and IETF
/// variant bits set.
pub fn offline_uuid(name: &str) -> String {
    let mut bytes = md5::compute(format!("OfflinePlayer:{name}")).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).hyphenated().to_string()
}

/// The UUID Floodgate gives a Bedrock player on a Java server: `new UUID(0, xuid)`.
pub fn floodgate_uuid(xuid: &str) -> Option<String> {
    let xuid: u64 = xuid.parse().ok()?;
    Some(uuid::Uuid::from_u64_pair(0, xuid).hyphenated().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuids_match_java() {
        assert_eq!(
            offline_uuid("Notch"),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_ne!(offline_uuid("notch"), offline_uuid("Notch"));
    }

    #[test]
    fn floodgate_uuids() {
        assert_eq!(
            floodgate_uuid("2535428692371234").as_deref(),
            Some("00000000-0000-0000-0009-01f57e8fe722")
        );
        assert_eq!(floodgate_uuid("x"), None);
    }
}

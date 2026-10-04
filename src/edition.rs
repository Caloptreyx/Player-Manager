//! Telling Java and Bedrock servers apart from the files in the server root.
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Edition {
    Java,
    Bedrock,
}

/// Files only a Java server creates.
const JAVA_FILES: [&str; 5] = [
    "ops.json",
    "banned-players.json",
    "banned-ips.json",
    "usercache.json",
    "eula.txt",
];

/// The edition of a server from the names of the (non-directory) files in its root and
/// `hints` (egg and image names). The Bedrock binary decides first, then Java's own files
/// and jars, then a "bedrock" hint or Bedrock's list files, then files both editions share.
pub fn detect(files: &[&str], hints: &[&str]) -> Option<Edition> {
    let has = |name: &str| files.contains(&name);

    if has("bedrock_server") || has("bedrock_server.exe") {
        return Some(Edition::Bedrock);
    }
    if JAVA_FILES.iter().any(|name| has(name))
        || files
            .iter()
            .any(|name| name.to_ascii_lowercase().ends_with(".jar"))
    {
        return Some(Edition::Java);
    }
    if hints
        .iter()
        .any(|hint| hint.to_ascii_lowercase().contains("bedrock"))
        || has("allowlist.json")
        || has("permissions.json")
    {
        return Some(Edition::Bedrock);
    }
    if has("whitelist.json") || has("server.properties") {
        return Some(Edition::Java);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedrock_binary_wins() {
        assert_eq!(
            detect(
                &["bedrock_server", "server.properties", "whitelist.json"],
                &[]
            ),
            Some(Edition::Bedrock)
        );
        assert_eq!(detect(&["bedrock_server.exe"], &[]), Some(Edition::Bedrock));
    }

    #[test]
    fn java_files_and_jars() {
        assert_eq!(detect(&["eula.txt"], &[]), Some(Edition::Java));
        assert_eq!(
            detect(&["paper-1.21.jar"], &["Bedrock"]),
            Some(Edition::Java)
        );
        assert_eq!(
            detect(&["ops.json", "permissions.json"], &[]),
            Some(Edition::Java)
        );
    }

    #[test]
    fn bedrock_lists_and_hints() {
        assert_eq!(
            detect(&["allowlist.json", "server.properties"], &[]),
            Some(Edition::Bedrock)
        );
        assert_eq!(
            detect(
                &["server.properties", "whitelist.json"],
                &["Vanilla Bedrock", "ghcr.io/x:debian"]
            ),
            Some(Edition::Bedrock)
        );
        assert_eq!(
            detect(&[], &["ghcr.io/x/bedrock:latest"]),
            Some(Edition::Bedrock)
        );
    }

    #[test]
    fn shared_files_mean_java() {
        assert_eq!(detect(&["server.properties"], &[]), Some(Edition::Java));
        assert_eq!(detect(&["whitelist.json"], &["Paper"]), Some(Edition::Java));
    }

    #[test]
    fn unknown_servers() {
        assert_eq!(detect(&[], &["Rust"]), None);
        assert_eq!(detect(&["config.yml", "start.sh"], &["Velocity"]), None);
    }
}

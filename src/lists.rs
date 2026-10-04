//! The JSON player lists (`whitelist.json`, `ops.json`, `allowlist.json`, `permissions.json`...):
//! arrays of objects, edited as `serde_json` values so fields this extension does not know
//! survive (and keep their order, `serde_json` is built with `preserve_order`).
use serde_json::Value;

pub type Entry = serde_json::Map<String, Value>;

/// The entries of a list file; blank files are empty lists.
pub fn parse(text: &str) -> Result<Vec<Value>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str(text) {
        Ok(Value::Array(entries)) => Ok(entries),
        Ok(_) => Err("expected a JSON array".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

/// Pretty JSON with two-space indentation, like the Gson output of the servers.
pub fn render(entries: &[Value]) -> String {
    serde_json::to_string_pretty(entries).expect("JSON values always serialize")
}

/// The object entries (other values are kept in the file but never shown or matched).
pub fn objects(entries: &[Value]) -> impl Iterator<Item = &Entry> {
    entries.iter().filter_map(Value::as_object)
}

pub fn string<'a>(entry: &'a Entry, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(Value::as_str)
}

/// A string field, or a number field as its decimal text (Bedrock tools write xuids both ways).
pub fn text(entry: &Entry, key: &str) -> Option<String> {
    match entry.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Whether the entry's `key` holds `name`, ignoring ASCII case.
pub fn has_name(entry: &Entry, key: &str, name: &str) -> bool {
    string(entry, key).is_some_and(|value| value.eq_ignore_ascii_case(name))
}

/// Updates the first entry `matches` accepts (dropping further matches, so an entry is never
/// listed twice) or appends a new one; `update` sets the fields.
pub fn upsert(
    entries: &mut Vec<Value>,
    matches: impl Fn(&Entry) -> bool,
    update: impl FnOnce(&mut Entry),
) {
    let is_match = |value: &Value| value.as_object().is_some_and(&matches);
    match entries.iter().position(is_match) {
        Some(index) => {
            let mut position = 0;
            entries.retain(|value| {
                let keep = position <= index || !is_match(value);
                position += 1;
                keep
            });
            if let Some(entry) = entries[index].as_object_mut() {
                update(entry);
            }
        }
        None => {
            let mut entry = Entry::new();
            update(&mut entry);
            entries.push(Value::Object(entry));
        }
    }
}

/// Removes every entry `matches` accepts; how many were removed.
pub fn remove(entries: &mut Vec<Value>, matches: impl Fn(&Entry) -> bool) -> usize {
    let before = entries.len();
    entries.retain(|value| !value.as_object().is_some_and(&matches));
    before - entries.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_lists() {
        assert_eq!(parse("").unwrap(), Vec::<Value>::new());
        assert_eq!(parse("\u{feff} \n").unwrap(), Vec::<Value>::new());
        assert_eq!(
            parse("[{\"name\":\"a\"}]").unwrap(),
            vec![json!({"name": "a"})]
        );
        assert!(parse("{}").is_err());
        assert!(parse("[{").is_err());
    }

    #[test]
    fn renders_gson_style() {
        assert_eq!(render(&[]), "[]");
        assert_eq!(
            render(&[json!({"uuid": "u", "name": "a"})]),
            "[\n  {\n    \"uuid\": \"u\",\n    \"name\": \"a\"\n  }\n]"
        );
    }

    #[test]
    fn upsert_updates_in_place_keeping_unknown_fields() {
        let mut entries = parse(
            r#"[{"uuid":"1","name":"alex","extra":true},{"uuid":"2","name":"Steve"},{"uuid":"3","name":"STEVE"},42]"#,
        )
        .unwrap();
        upsert(
            &mut entries,
            |entry| has_name(entry, "name", "steve"),
            |entry| {
                entry.insert("uuid".into(), json!("9"));
                entry.insert("name".into(), json!("Steve"));
            },
        );
        assert_eq!(
            entries,
            vec![
                json!({"uuid": "1", "name": "alex", "extra": true}),
                json!({"uuid": "9", "name": "Steve"}),
                json!(42),
            ]
        );
        upsert(
            &mut entries,
            |entry| string(entry, "uuid") == Some("1"),
            |entry| {
                entry.insert("name".into(), json!("Alex"));
            },
        );
        assert_eq!(
            render(&entries[..1]),
            "[\n  {\n    \"uuid\": \"1\",\n    \"name\": \"Alex\",\n    \"extra\": true\n  }\n]"
        );
    }

    #[test]
    fn upsert_appends_new_entries_in_field_order() {
        let mut entries = Vec::new();
        upsert(
            &mut entries,
            |entry| has_name(entry, "name", "a"),
            |entry| {
                entry.insert("uuid".into(), json!("u"));
                entry.insert("name".into(), json!("a"));
            },
        );
        assert_eq!(
            render(&entries),
            "[\n  {\n    \"uuid\": \"u\",\n    \"name\": \"a\"\n  }\n]"
        );
    }

    #[test]
    fn removes_matches() {
        let mut entries = parse(r#"[{"name":"A"},{"name":"b"},{"name":"a"},"a"]"#).unwrap();
        assert_eq!(
            remove(&mut entries, |entry| has_name(entry, "name", "a")),
            2
        );
        assert_eq!(entries, vec![json!({"name": "b"}), json!("a")]);
        assert_eq!(
            remove(&mut entries, |entry| has_name(entry, "name", "zed")),
            0
        );
    }

    #[test]
    fn reads_numeric_and_string_xuids() {
        let entries = parse(r#"[{"xuid":"123"},{"xuid":456},{"xuid":null}]"#).unwrap();
        let xuids: Vec<_> = objects(&entries).map(|entry| text(entry, "xuid")).collect();
        assert_eq!(xuids, vec![Some("123".into()), Some("456".into()), None]);
    }
}

//! Reading and in-place editing of `server.properties`.

/// The key of a `key=value` (or `key: value`) line and the byte offset its value starts at;
/// `None` for blank and comment lines.
fn split_line(line: &str) -> Option<(&str, usize)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with(['#', '!']) {
        return None;
    }
    let indent = line.len() - trimmed.len();
    let separator = trimmed.find(['=', ':'])?;
    let key = trimmed[..separator].trim_end();
    let after = &trimmed[separator + 1..];
    let value_start = indent + separator + 1 + (after.len() - after.trim_start().len());
    Some((key, value_start))
}

/// The line without its `\n` / `\r\n` ending.
fn body(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

/// The value of `key` (the last one when it repeats).
pub fn get<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content
        .lines()
        .filter_map(|line| {
            let (found, start) = split_line(line)?;
            (found == key).then(|| line[start..].trim_end())
        })
        .last()
}

pub fn get_bool(content: &str, key: &str) -> Option<bool> {
    match get(content, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub fn get_u32(content: &str, key: &str) -> Option<u32> {
    get(content, key)?.parse().ok()
}

pub fn has(content: &str, key: &str) -> bool {
    get(content, key).is_some()
}

/// `content` with the value of every `key` line replaced by `value`, or `key=value` appended
/// when the key is missing; every other byte is kept.
pub fn set(content: &str, key: &str, value: &str) -> String {
    let mut output = String::with_capacity(content.len() + key.len() + value.len() + 2);
    let mut replaced = false;
    for line in content.split_inclusive('\n') {
        let text = body(line);
        match split_line(text) {
            Some((found, start)) if found == key => {
                output.push_str(&text[..start]);
                output.push_str(value);
                output.push_str(&line[text.len()..]);
                replaced = true;
            }
            _ => output.push_str(line),
        }
    }
    if !replaced {
        let newline = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        if !content.is_empty() && !content.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push_str(newline);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "#Minecraft server properties\n#Sat Oct 04 12:00:00 UTC 2026\nonline-mode=false\nwhite-list = true\nmax-players=20\nmotd=A: B=C\n";

    #[test]
    fn reads_values() {
        assert_eq!(get_bool(FILE, "online-mode"), Some(false));
        assert_eq!(get_bool(FILE, "white-list"), Some(true));
        assert_eq!(get_u32(FILE, "max-players"), Some(20));
        assert_eq!(get(FILE, "motd"), Some("A: B=C"));
        assert_eq!(get(FILE, "allow-list"), None);
        assert_eq!(
            get_u32("op-permission-level=x\n", "op-permission-level"),
            None
        );
        assert_eq!(get("a=1\na=2\n", "a"), Some("2"));
        assert_eq!(get("#a=1\n", "a"), None);
    }

    #[test]
    fn replaces_only_the_value() {
        let edited = set(FILE, "white-list", "false");
        assert_eq!(
            edited,
            FILE.replace("white-list = true", "white-list = false")
        );
        assert_eq!(set(FILE, "online-mode", "false"), FILE);
    }

    #[test]
    fn keeps_crlf_and_missing_final_newline() {
        assert_eq!(
            set("a=1\r\nwhite-list=false\r\nb=2", "white-list", "true"),
            "a=1\r\nwhite-list=true\r\nb=2"
        );
        assert_eq!(
            set("a=1\r\nb=2", "allow-list", "true"),
            "a=1\r\nb=2\r\nallow-list=true\r\n"
        );
    }

    #[test]
    fn appends_missing_keys() {
        assert_eq!(set("a=1\n", "white-list", "true"), "a=1\nwhite-list=true\n");
        assert_eq!(set("", "white-list", "true"), "white-list=true\n");
        assert_eq!(
            set("white-list=\n", "white-list", "true"),
            "white-list=true\n"
        );
    }
}

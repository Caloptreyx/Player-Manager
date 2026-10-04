//! Reading the console: ANSI and log prefix stripping, finding the lines printed after a
//! command was sent, and parsing `list` answers and Bedrock join lines.

/// The log as lines without ANSI escapes, Minecraft color codes and carriage returns.
pub fn split_lines(log: &str) -> Vec<String> {
    log.lines().map(strip_ansi).collect()
}

/// `line` without ANSI escape sequences (CSI, OSC and two-byte escapes), `§x` color codes and
/// other control characters.
pub fn strip_ansi(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    // parameters and intermediates, up to the final byte (@ to ~)
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // up to BEL or ST (ESC \)
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            },
            '§' => {
                chars.next();
            }
            c if c.is_control() => {}
            c => output.push(c),
        }
    }
    output
}

/// The message of a log line: without leading `[...]` groups that look like log prefixes
/// (they contain `:` or `/`, which player names cannot), and the `: ` after them.
pub fn message(line: &str) -> &str {
    let mut rest = line.trim_start();
    rest = rest.strip_prefix("NO LOG FILE! - ").unwrap_or(rest);
    let mut stripped = false;
    while rest.starts_with('[') {
        let Some(end) = rest.find(']') else { break };
        if !rest[..end].contains([':', '/']) {
            break;
        }
        rest = rest[end + 1..].trim_start();
        stripped = true;
    }
    if stripped && let Some(after) = rest.strip_prefix(':') {
        rest = after.trim_start();
    }
    rest
}

/// The lines of `current` that come after `baseline`: `current` is a later tail of the same
/// log, so the longest suffix of `baseline` that starts `current` is the part both saw.
pub fn new_lines<'a>(baseline: &[String], current: &'a [String]) -> &'a [String] {
    let longest = baseline.len().min(current.len());
    for overlap in (1..=longest).rev() {
        if baseline[baseline.len() - overlap..] == current[..overlap] {
            return &current[overlap..];
        }
    }
    current
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListAnswer {
    pub count: u32,
    pub max: u32,
    /// Names with the UUID `list uuids` adds.
    pub players: Vec<(String, Option<String>)>,
}

/// `N of a max of M players online: names` (modern Java) or `N/M players online:` (legacy
/// Java and Bedrock, names on the next line).
enum Header<'a> {
    Inline(u32, u32, &'a str),
    NextLine(u32, u32),
}

fn number(text: &str) -> Option<(u32, &str)> {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    Some((text[..end].parse().ok()?, &text[end..]))
}

fn header(message: &str) -> Option<Header<'_>> {
    let (count, rest) = number(message.strip_prefix("There are ")?)?;
    if let Some(rest) = rest.strip_prefix(" of a max of ") {
        let (max, rest) = number(rest)?;
        let names = rest.strip_prefix(" players online:")?;
        return Some(Header::Inline(count, max, names));
    }
    let (max, rest) = number(rest.strip_prefix('/')?)?;
    let names = rest.strip_prefix(" players online:")?;
    if names.trim().is_empty() {
        Some(Header::NextLine(count, max))
    } else {
        Some(Header::Inline(count, max, names))
    }
}

/// `a, b (uuid)` as names with optional UUIDs.
fn players(names: &str) -> Vec<(String, Option<String>)> {
    names
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            if let Some(open) = name.rfind(" (")
                && let Some(uuid) = name[open + 2..].strip_suffix(')')
                && let Some(uuid) = crate::validate::java_id(uuid)
            {
                return (name[..open].to_string(), Some(uuid));
            }
            (name.to_string(), None)
        })
        .collect()
}

/// The first complete `list` answer in `lines` (`None` while a `N/M` header still waits for
/// its names line).
pub fn parse_list(lines: &[String]) -> Option<ListAnswer> {
    for (index, line) in lines.iter().enumerate() {
        let Some(found) = header(message(line)) else {
            continue;
        };
        let (count, max, names) = match found {
            Header::Inline(count, max, names) => (count, max, names),
            Header::NextLine(count, max) if count == 0 => (count, max, ""),
            Header::NextLine(count, max) => (count, max, message(lines.get(index + 1)?)),
        };
        return Some(ListAnswer {
            count,
            max,
            players: players(names),
        });
    }
    None
}

/// `(name, xuid)` of every Bedrock `Player connected: <name>, xuid: <n>` line, oldest first.
pub fn connected_players(lines: &[String]) -> Vec<(String, String)> {
    const MARKER: &str = "Player connected: ";
    lines
        .iter()
        .filter_map(|line| {
            let rest = &line[line.find(MARKER)? + MARKER.len()..];
            let (name, rest) = rest.split_once(", xuid: ")?;
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            let xuid = &rest[..end];
            (crate::validate::bedrock_name(name) && crate::validate::bedrock_id(xuid))
                .then(|| (name.to_string(), xuid.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn strips_ansi_and_color_codes() {
        assert_eq!(
            strip_ansi("\u{1b}[0;32m[12:00:00 INFO]: \u{1b}[mThere are §a1§r\r"),
            "[12:00:00 INFO]: There are 1"
        );
        assert_eq!(
            strip_ansi("\u{1b}]0;title\u{7}ok\u{1b}]8;;x\u{1b}\\!"),
            "ok!"
        );
    }

    #[test]
    fn strips_log_prefixes() {
        assert_eq!(message("[12:00:00 INFO]: There are"), "There are");
        assert_eq!(
            message("[12:00:00] [Server thread/INFO]: There are"),
            "There are"
        );
        assert_eq!(
            message("[12:00:00] [Server thread/INFO] [minecraft/DedicatedServer]: There are"),
            "There are"
        );
        assert_eq!(
            message("[2026-01-01 12:00:00:000 INFO] There are"),
            "There are"
        );
        assert_eq!(message("NO LOG FILE! - [2026-01-01 12:00:00 INFO] x"), "x");
        assert_eq!(message("Steve, Alex"), "Steve, Alex");
        // `/say` output keeps its sender, so chat cannot fake an answer
        assert_eq!(
            message("[12:00:00 INFO]: [Steve] There are 9 of a max of 9 players online: x"),
            "[Steve] There are 9 of a max of 9 players online: x"
        );
    }

    #[test]
    fn parses_modern_java_answers() {
        let lines = owned(&[
            "[12:00:00 INFO]: <Steve> There are 5 of a max of 5 players online: fake",
            "[12:00:00] [Server thread/INFO]: There are 2 of a max of 20 players online: Notch (069a79f4-44e9-4726-a5be-fca90e38aaf5), jeb_",
        ]);
        assert_eq!(
            parse_list(&lines),
            Some(ListAnswer {
                count: 2,
                max: 20,
                players: vec![
                    (
                        "Notch".into(),
                        Some("069a79f4-44e9-4726-a5be-fca90e38aaf5".into())
                    ),
                    ("jeb_".into(), None),
                ],
            })
        );
    }

    #[test]
    fn parses_empty_answers() {
        let modern = owned(&["[12:00:00 INFO]: There are 0 of a max of 20 players online: "]);
        let legacy = owned(&["[12:00:00] [Server thread/INFO]: There are 0/20 players online:"]);
        let bedrock = owned(&["[2026-01-01 12:00:00:000 INFO] There are 0/10 players online:"]);
        for lines in [modern, legacy] {
            assert_eq!(
                parse_list(&lines),
                Some(ListAnswer {
                    count: 0,
                    max: 20,
                    players: vec![]
                })
            );
        }
        assert_eq!(
            parse_list(&bedrock),
            Some(ListAnswer {
                count: 0,
                max: 10,
                players: vec![]
            })
        );
    }

    #[test]
    fn parses_next_line_answers() {
        let legacy = owned(&[
            "[12:00:00] [Server thread/INFO]: There are 2/20 players online:",
            "[12:00:00] [Server thread/INFO]: Notch, jeb_",
        ]);
        let bedrock = owned(&["There are 2/10 players online:", "Some Guy, Alex"]);
        assert_eq!(
            parse_list(&legacy).map(|answer| answer.players),
            Some(vec![("Notch".into(), None), ("jeb_".into(), None)])
        );
        assert_eq!(
            parse_list(&bedrock),
            Some(ListAnswer {
                count: 2,
                max: 10,
                players: vec![("Some Guy".into(), None), ("Alex".into(), None)],
            })
        );
        // the names line has not been printed yet
        assert_eq!(
            parse_list(&owned(&["There are 1/10 players online:"])),
            None
        );
        assert_eq!(parse_list(&owned(&["[12:00:00 INFO]: Done (3.2s)!"])), None);
    }

    #[test]
    fn finds_new_lines_after_the_baseline() {
        let baseline = owned(&["a", "b", "c"]);
        assert_eq!(
            new_lines(&baseline, &owned(&["b", "c", "d"])),
            owned(&["d"])
        );
        assert_eq!(
            new_lines(&baseline, &owned(&["a", "b", "c", "d", "e"])),
            owned(&["d", "e"])
        );
        assert!(new_lines(&baseline, &baseline).is_empty());
        assert_eq!(
            new_lines(&baseline, &owned(&["x", "y"])),
            owned(&["x", "y"])
        );
        assert_eq!(new_lines(&[], &owned(&["x"])), owned(&["x"]));
        // an older answer still in the window is not new
        let baseline = owned(&["There are 0/10 players online:", "b"]);
        let current = owned(&[
            "There are 0/10 players online:",
            "b",
            "There are 1/10 players online:",
            "Alex",
        ]);
        assert_eq!(
            new_lines(&baseline, &current),
            owned(&["There are 1/10 players online:", "Alex"])
        );
    }

    #[test]
    fn parses_bedrock_joins() {
        let lines = owned(&[
            "[2026-01-01 12:00:00:000 INFO] Player connected: Some Guy, xuid: 2535428692371234",
            "[2026-01-01 12:00:01:000 INFO] Player connected: Alex, xuid: 2535400000000001, pfid: abc",
            "[2026-01-01 12:00:02:000 INFO] Player disconnected: Alex, xuid: 2535400000000001",
            "[2026-01-01 12:00:03:000 INFO] Player connected: Bad_Name, xuid: 1",
        ]);
        assert_eq!(
            connected_players(&lines),
            vec![
                ("Some Guy".into(), "2535428692371234".into()),
                ("Alex".into(), "2535400000000001".into()),
            ]
        );
    }
}

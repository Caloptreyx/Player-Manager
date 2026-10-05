//! Reading the console: ANSI stripping and finding the lines printed after a command was sent.

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
}

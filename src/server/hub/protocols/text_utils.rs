//! Shared text utilities for protocol prompt generation.

/// Collapse consecutive blank lines into a single blank line
/// and trim trailing blank lines.
pub fn collapse_blank_lines(s: &str) -> String {
    let mut result = Vec::new();
    let mut prev_blank = false;
    for line in s.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push(line);
        prev_blank = is_blank;
    }
    while result.last().is_some_and(|l| l.trim().is_empty()) {
        result.pop();
    }
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_blank_lines_removes_consecutive_blanks() {
        let input = "a\n\n\nb";
        assert_eq!(collapse_blank_lines(input), "a\n\nb");
    }

    #[test]
    fn collapse_blank_lines_preserves_single_blanks() {
        let input = "a\n\nb";
        assert_eq!(collapse_blank_lines(input), "a\n\nb");
    }

    #[test]
    fn collapse_blank_lines_trims_trailing_blanks() {
        let input = "a\n\n";
        assert_eq!(collapse_blank_lines(input), "a");
    }

    #[test]
    fn collapse_blank_lines_empty_input() {
        assert_eq!(collapse_blank_lines(""), "");
    }
}

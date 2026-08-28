//! Shared text utilities for protocol prompt generation.

/// Strip HTML/XML comments and collapse the blank lines they leave behind.
///
/// Prompt files carry their reasoning inline — why a rule exists, what the
/// previous version got wrong, which code path a claim was checked against.
/// That is the most valuable thing in them for whoever edits next, and it is
/// worth nothing to the model, which pays for every token of it on every
/// call. `config/runtime_agent/system.md` alone ships out once per agent per
/// step.
///
/// Deliberately not a parser: prompt comments are `<!-- ... -->` at the top
/// level and never nested, so a scan for the next `-->` is exact for the
/// input this handles.
///
/// # Panics
///
/// On an unterminated `<!--`. There are only two ways to treat one and both
/// of the quiet ones are worse than stopping: emit it and the file's private
/// reasoning ships to the model as instructions, or drop to end-of-file and a
/// missing `-->` silently deletes the rest of a prompt — a whole `<examples>`
/// block gone with nothing to see. Prompts are `include_str!` constants, so
/// this fires on the first call in a run, for everyone, with the offset to
/// fix. It cannot be triggered by user input.
pub fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;

    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);

        let offset = rest[start..].find("-->").unwrap_or_else(|| {
            let absolute = s.len() - rest.len() + start;
            let line = s[..absolute].lines().count();
            let preview: String = rest[start..].chars().take(60).collect();
            panic!(
                "unterminated `<!--` in prompt text at byte {absolute} (line {line}): \
                 {preview:?} — every comment needs a closing `-->`"
            );
        });

        rest = &rest[start + offset + "-->".len()..];
    }
    out.push_str(rest);

    collapse_blank_lines(&out)
}

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

    #[test]
    fn strip_comments_removes_a_block_comment() {
        let input = "<role>\n<!-- why this exists -->\nYou are an agent.\n</role>";
        assert_eq!(
            strip_comments(input),
            "<role>\n\nYou are an agent.\n</role>"
        );
    }

    #[test]
    fn strip_comments_removes_a_multiline_comment() {
        let input = "a\n<!-- line one\n     line two\n     line three -->\nb";
        assert_eq!(strip_comments(input), "a\n\nb");
    }

    #[test]
    fn strip_comments_removes_several_and_collapses_the_gap() {
        let input = "<x>\n<!-- one -->\n<!-- two -->\ntext\n</x>";
        assert_eq!(strip_comments(input), "<x>\n\ntext\n</x>");
    }

    #[test]
    fn strip_comments_leaves_text_without_comments_alone() {
        let input = "<role>\nYou are an agent.\n</role>";
        assert_eq!(strip_comments(input), input);
    }

    /// A comment body containing XML must not end the comment early — prompt
    /// comments quote the tags they describe constantly.
    #[test]
    fn strip_comments_handles_tags_inside_a_comment() {
        let input = "<!-- <output> is last on purpose -->\nkept";
        assert_eq!(strip_comments(input), "\nkept");
    }

    #[test]
    #[should_panic(expected = "unterminated `<!--`")]
    fn strip_comments_panics_on_an_unterminated_comment() {
        strip_comments("<role>\nfine\n<!-- forgot to close\nYou are an agent.");
    }
}

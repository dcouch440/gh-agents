//! Syntax highlighting for file content.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Syntax highlighter using syntect for language detection and highlighting.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default syntax definitions and themes.
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight file content based on file extension.
    ///
    /// Returns a vector of ratatui Lines with syntax highlighting applied.
    pub fn highlight_file(&self, path: &Path, content: &str) -> Vec<Line<'static>> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        content
            .lines()
            .map(|line| {
                let ranges = highlighter
                    .highlight_line(line, &self.syntax_set)
                    .unwrap_or_default();

                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                    })
                    .collect();

                if spans.is_empty() {
                    Line::from(Span::raw(line.to_string()))
                } else {
                    Line::from(spans)
                }
            })
            .collect()
    }

    /// Get the list of supported file extensions.
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.syntax_set
            .syntaxes()
            .iter()
            .flat_map(|s| s.file_extensions.iter().map(|e| e.as_str()))
            .collect()
    }

    /// Check if an extension is supported for highlighting.
    pub fn is_extension_supported(&self, extension: &str) -> bool {
        self.syntax_set
            .find_syntax_by_extension(extension)
            .is_some()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert syntect style to ratatui style.
fn syntect_to_ratatui_style(style: SyntectStyle) -> Style {
    Style::default().fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighter_creates_successfully() {
        let highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.supported_extensions().is_empty());
    }

    #[test]
    fn highlighter_default_same_as_new() {
        let h1 = SyntaxHighlighter::new();
        let h2 = SyntaxHighlighter::default();
        assert_eq!(
            h1.supported_extensions().len(),
            h2.supported_extensions().len()
        );
    }

    #[test]
    fn rust_extension_supported() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_extension_supported("rs"));
    }

    #[test]
    fn javascript_extension_supported() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_extension_supported("js"));
    }

    #[test]
    fn markdown_extension_supported() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_extension_supported("md"));
    }

    #[test]
    fn python_extension_supported() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_extension_supported("py"));
    }

    #[test]
    fn unknown_extension_returns_false() {
        let highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.is_extension_supported("xyz123unknown"));
    }

    #[test]
    fn highlight_rust_code() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let path = Path::new("test.rs");

        let lines = highlighter.highlight_file(path, code);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn highlight_empty_content() {
        let highlighter = SyntaxHighlighter::new();
        let path = Path::new("test.rs");

        let lines = highlighter.highlight_file(path, "");
        assert_eq!(lines.len(), 0); // Empty string has no lines
    }

    #[test]
    fn highlight_plain_text() {
        let highlighter = SyntaxHighlighter::new();
        let path = Path::new("test.txt");
        let content = "Just some plain text\nNo special highlighting";

        let lines = highlighter.highlight_file(path, content);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn highlight_unknown_extension_uses_plain_text() {
        let highlighter = SyntaxHighlighter::new();
        let path = Path::new("file.unknownext");
        let content = "Some content";

        let lines = highlighter.highlight_file(path, content);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn highlight_file_without_extension() {
        let highlighter = SyntaxHighlighter::new();
        let path = Path::new("Makefile");
        let content = "all:\n\techo hello";

        let lines = highlighter.highlight_file(path, content);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn syntect_to_ratatui_conversion() {
        let syntect_style = SyntectStyle {
            foreground: syntect::highlighting::Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: syntect::highlighting::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        let ratatui_style = syntect_to_ratatui_style(syntect_style);
        assert_eq!(ratatui_style.fg, Some(Color::Rgb(255, 128, 64)));
    }
}

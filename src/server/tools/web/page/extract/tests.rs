#[cfg(test)]
mod tests {
    use super::super::*;

    const ARTICLE: &str = r#"<!doctype html><html><head><title>Async Rust</title></head>
<body><nav>Home About Contact</nav>
<article><h1>The State of Async Rust</h1>
<p>Tokio is the most widely used async runtime in the Rust ecosystem, and most
libraries assume it. That assumption is the single biggest source of friction
when you try to use anything else, because the ecosystem splits along runtime
lines rather than along problem lines.</p>
<p>The suggested replacement is smol, which is lightweight and much more
explicit about what it is doing. Whether that explicitness is worth the smaller
ecosystem depends entirely on what you are building and who maintains it.</p>
</article><footer>© 2026</footer></body></html>"#;

    #[tokio::test]
    async fn an_article_page_extracts_its_main_content() {
        let e = html_to_markdown(ARTICLE.to_string(), "https://example.com/a".into()).await;
        assert!(
            e.markdown.contains("Tokio is the most widely used"),
            "{e:?}"
        );
        assert!(e.markdown.contains("smol"), "{e:?}");
    }

    #[tokio::test]
    async fn boilerplate_is_dropped() {
        let e = html_to_markdown(ARTICLE.to_string(), "https://example.com/a".into()).await;
        assert!(
            !e.markdown.contains("Home About Contact"),
            "nav survived: {e:?}"
        );
    }

    #[tokio::test]
    async fn the_title_is_recovered() {
        let e = html_to_markdown(ARTICLE.to_string(), "https://example.com/a".into()).await;
        assert!(
            e.title.as_deref().unwrap_or("").contains("Async Rust"),
            "{:?}",
            e.title
        );
    }

    // A listing or landing page has no article; converting the whole body is
    // worse but honest, and the method must say so.
    #[tokio::test]
    async fn a_page_without_an_article_falls_back_to_whole_page() {
        let html = "<html><head><title>Links</title></head><body>\
            <ul><li><a href='/a'>One</a></li><li><a href='/b'>Two</a></li></ul>\
            </body></html>";
        let e = html_to_markdown(html.to_string(), "https://example.com/".into()).await;
        assert_eq!(e.method, Method::WholePage);
        assert_eq!(e.title.as_deref(), Some("Links"));
    }

    #[tokio::test]
    async fn malformed_html_does_not_panic() {
        for html in [
            "<html><body><p>unclosed",
            "<<<>>>",
            "",
            "<html><body>\u{0}\u{1}</body></html>",
        ] {
            let e = html_to_markdown(html.to_string(), "https://e.com/".into()).await;
            // No assertion on content — the point is that it returns at all.
            let _ = e.markdown;
        }
    }

    #[tokio::test]
    async fn a_bad_document_url_is_not_fatal() {
        let e = html_to_markdown(ARTICLE.to_string(), "not a url".into()).await;
        assert!(!e.markdown.trim().is_empty(), "content lost to a bad url");
    }

    #[tokio::test]
    async fn multibyte_content_survives_extraction() {
        let html = "<html><body><article><p>日本語のテキストです。これは十分に長い段落で、\
                    リーダビリティが本文として認識するのに必要な長さがあります。もう少し\
                    続けて書きます。</p></article></body></html>";
        let e = html_to_markdown(html.to_string(), "https://e.com/".into()).await;
        assert!(e.markdown.contains("日本語"), "{e:?}");
    }

    #[test]
    fn a_title_tag_with_attributes_is_still_found() {
        let t = title_from_html(r#"<html><head><title dir="ltr"> Spaced </title></head></html>"#);
        assert_eq!(t.as_deref(), Some("Spaced"));
    }

    #[test]
    fn a_missing_or_empty_title_is_none() {
        assert_eq!(title_from_html("<html><body>x</body></html>"), None);
        assert_eq!(title_from_html("<title>   </title>"), None);
        assert_eq!(title_from_html("<title>unclosed"), None);
    }

    // A single-page app returns a near-empty body with a large script bundle.
    // Saying so beats handing the agent a nav bar and a spinner.
    #[test]
    fn a_large_page_yielding_no_text_is_flagged_as_a_js_shell() {
        let e = Extracted {
            markdown: "Loading...".into(),
            ..Default::default()
        };
        assert!(looks_like_js_shell(&e, 50_000));
    }

    #[test]
    fn a_genuinely_short_page_is_not_flagged() {
        let e = Extracted {
            markdown: "Short.".into(),
            ..Default::default()
        };
        assert!(!looks_like_js_shell(&e, 400), "small page misflagged");
    }

    #[test]
    fn a_page_with_real_content_is_not_flagged() {
        let e = Extracted {
            markdown: "x".repeat(500),
            ..Default::default()
        };
        assert!(!looks_like_js_shell(&e, 100_000));
    }

    #[test]
    fn method_labels_are_distinguishable() {
        assert_ne!(Method::Readability.label(), Method::WholePage.label());
    }
}

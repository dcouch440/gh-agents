#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    fn parse(v: serde_json::Value) -> BraveResponse {
        serde_json::from_value(v).expect("response should deserialize")
    }

    fn one_result() -> serde_json::Value {
        json!({
            "query": {"original": "rust async", "more_results_available": true},
            "web": {"results": [{
                "title": "The State of <strong>Async</strong> Rust",
                "url": "https://corrode.dev/blog/async/",
                "description": "The suggested\n replacement is smol,\twhich is lightweight.",
                "age": "1 month ago",
                "page_age": "2026-07-30T00:00:00",
                "meta_url": {"hostname": "corrode.dev"},
                "extra_snippets": ["first snippet", "second snippet", "third snippet"]
            }]}
        })
    }

    // A live search for an ordinary query returns only query/web/mixed/type/
    // videos — no faq, infobox, news, discussions or summarizer. Required
    // fields would fail to deserialize on the common case.
    #[test]
    fn a_response_missing_every_optional_section_still_parses() {
        let r = parse(json!({"query": {"original": "x"}, "web": {"results": []}}));
        assert!(r.web.is_some());
    }

    #[test]
    fn a_response_with_no_web_section_parses() {
        let r = parse(json!({"query": {"original": "x"}}));
        assert!(r.web.is_none());
    }

    #[test]
    fn a_result_missing_every_optional_field_parses() {
        let r = parse(json!({"web": {"results": [{"title": "t", "url": "u"}]}}));
        let res = &r.web.unwrap().results[0];
        assert!(res.description.is_none());
        assert!(res.meta_url.is_none());
        assert!(res.extra_snippets.is_none());
    }

    // Golden test: pins the exact text the model reads.
    #[test]
    fn the_rendered_report_is_labeled_and_scannable() {
        let out = render("rust async", &parse(one_result()), None, None);
        let expected = "\
result: success
query: rust async
results: 1

results:
  [1] The State of Async Rust
      https://corrode.dev/blog/async/
      corrode.dev · 1 month ago
      The suggested replacement is smol, which is lightweight.
      - first snippet
      - second snippet

Results are search snippets, not sources. Use read_webpage on a URL above before relying on what it says.
";
        assert_eq!(out, expected, "\n--- got ---\n{out}");
    }

    #[test]
    fn brave_highlight_markup_is_stripped_from_titles() {
        let out = render("rust async", &parse(one_result()), None, None);
        assert!(!out.contains("<strong>"), "{out}");
    }

    // Descriptions arrive with embedded newlines and tabs; leaving them in
    // would break the indented layout.
    #[test]
    fn whitespace_in_descriptions_is_collapsed() {
        let out = render("rust async", &parse(one_result()), None, None);
        assert!(out.contains("The suggested replacement is smol, which is lightweight."));
    }

    #[test]
    fn only_the_first_two_snippets_are_rendered() {
        let out = render("rust async", &parse(one_result()), None, None);
        assert!(out.contains("first snippet"));
        assert!(out.contains("second snippet"));
        assert!(!out.contains("third snippet"), "snippets should be capped");
    }

    // An empty result set is a fact, not a failure. Returning an error value
    // would count toward the engine's repeated-failure breaker.
    #[test]
    fn no_results_is_a_successful_report_not_an_error() {
        let out = render(
            "obscure",
            &parse(json!({"web": {"results": []}})),
            None,
            None,
        );
        assert!(out.starts_with("result: success\n"), "{out}");
        assert!(out.contains("No results"), "{out}");
    }

    // A quota that has run out is exactly when results stop coming back, so
    // the warning has to survive the empty-results early return.
    #[test]
    fn a_low_quota_is_reported_even_when_there_are_no_results() {
        let out = render(
            "obscure",
            &parse(json!({"web": {"results": []}})),
            Some(3),
            None,
        );
        assert!(out.contains("3 searches left this month"), "{out}");
        assert!(out.contains("No results"), "{out}");
    }

    #[test]
    fn brave_flagging_weak_results_is_surfaced() {
        let mut v = one_result();
        v["query"]["bad_results"] = json!(true);
        let out = render("q", &parse(v), None, None);
        assert!(out.contains("quality: low"), "{out}");
    }

    #[test]
    fn a_healthy_quota_is_not_mentioned() {
        let out = render("q", &parse(one_result()), Some(1500), None);
        assert!(!out.contains("quota"), "{out}");
    }

    #[test]
    fn a_nearly_exhausted_quota_is_warned_about() {
        let out = render("q", &parse(one_result()), Some(12), None);
        assert!(out.contains("12 searches left this month"), "{out}");
    }

    #[test]
    fn a_result_without_a_hostname_or_age_still_renders() {
        let v = json!({"web": {"results": [{"title": "t", "url": "https://e.com"}]}});
        let out = render("q", &parse(v), None, None);
        assert!(out.contains("[1] t"), "{out}");
        assert!(out.contains("https://e.com"), "{out}");
    }

    #[test]
    fn long_descriptions_are_truncated() {
        let long = "x".repeat(1000);
        let v = json!({"web": {"results": [{
            "title": "t", "url": "u", "description": long
        }]}});
        let out = render("q", &parse(v), None, None);
        assert!(
            out.len() < 900,
            "description was not truncated: {}",
            out.len()
        );
    }

    // ── quota header parsing ───────────────────────────────────────────────

    fn headers(value: &str) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("x-ratelimit-remaining", value.parse().unwrap());
        h
    }

    #[test]
    fn the_month_window_is_the_second_value() {
        // Brave reports "<per-second>, <per-month>".
        assert_eq!(remaining_month_quota(&headers("0, 1999")), Some(1999));
        assert_eq!(remaining_month_quota(&headers("1, 42")), Some(42));
    }

    #[test]
    fn a_missing_or_malformed_quota_header_is_simply_absent() {
        assert_eq!(
            remaining_month_quota(&reqwest::header::HeaderMap::new()),
            None
        );
        assert_eq!(remaining_month_quota(&headers("5")), None);
        assert_eq!(remaining_month_quota(&headers("a, b")), None);
    }

    // ── freshness validation ───────────────────────────────────────────────

    #[test]
    fn only_documented_freshness_values_are_forwarded() {
        for v in ["pd", "pw", "pm", "py"] {
            assert!(is_valid_freshness(v), "{v} should be accepted");
        }
        // An invalid value makes Brave reject the whole request, so a
        // slightly-wrong parameter would become a failed search.
        for v in ["yesterday", "1d", "", "PD", "p"] {
            assert!(!is_valid_freshness(v), "{v} should be rejected");
        }
    }

    // ── guards ─────────────────────────────────────────────────────────────

    // An unrecognised freshness must not be dropped: an unfiltered search
    // rendered without comment reads as a recency-filtered one.
    #[tokio::test]
    async fn an_invalid_freshness_is_refused_rather_than_ignored() {
        let v = execute(&json!({"query": "rust", "freshness": "2026-01-01to2026-06-01"})).await;
        let err = v
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert!(err.contains("Invalid freshness"), "{v}");
    }

    #[test]
    fn an_accepted_freshness_is_echoed_in_the_report() {
        let out = render("q", &parse(one_result()), None, Some("pw"));
        assert!(out.contains("freshness: pw"), "{out}");
    }

    #[tokio::test]
    async fn a_missing_query_is_a_tool_error() {
        let v = execute(&json!({})).await;
        assert!(v.get("error").is_some(), "{v}");
        let v = execute(&json!({"query": "   "})).await;
        assert!(v.get("error").is_some(), "{v}");
    }
}

#[cfg(test)]
mod live_tests {
    use super::super::*;
    use serde_json::json;

    /// Hits the real Brave API. Requires BRAVE_SEARCH_API_KEY and spends one
    /// query from the monthly quota, so it is opt-in:
    ///
    /// ```text
    /// NEXOR_WEB_EGRESS_MODE=direct cargo test -- --ignored brave_live
    /// ```
    #[tokio::test]
    #[ignore = "hits the live Brave API and spends quota"]
    async fn brave_live_search_returns_a_rendered_report() {
        // This is the one test that installs the process-wide policy, which is
        // why it is `#[ignore]`d: it is opt-in and runs alone. `install` is
        // first-writer-wins, so if anything else got there first this test
        // would silently exercise that policy instead of the one below.
        let installed = crate::net::egress::install(crate::net::egress::EgressConfig {
            mode: crate::net::egress::EgressMode::parse(
                std::env::var("NEXOR_WEB_EGRESS_MODE").ok().as_deref(),
            ),
            proxy_url: std::env::var("NEXOR_VPN_PROXY_URL").ok(),
            is_production: false,
        });
        assert!(
            installed,
            "an egress policy was already installed; run this test on its own"
        );

        let v = execute(&json!({"query": "rust async runtime comparison"})).await;
        let s = v
            .as_str()
            .unwrap_or_else(|| panic!("expected a rendered report, got {v}"));
        assert!(s.starts_with("result: success\n"), "{s}");
        assert!(s.contains("results:"), "{s}");
        assert!(s.contains("https://"), "no URLs in report:\n{s}");
        println!("\n{s}");
    }
}

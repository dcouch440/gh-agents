#[cfg(test)]
mod tests {
    use super::super::*;

    fn rejected(raw: &str) -> UrlRejection {
        validate(raw).unwrap_err()
    }

    #[test]
    fn ordinary_https_urls_are_accepted() {
        assert!(validate("https://example.com/a/b?c=d").is_ok());
        assert!(validate("http://example.com").is_ok());
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        assert!(validate("  https://example.com  ").is_ok());
    }

    #[test]
    fn the_query_is_preserved_and_the_fragment_dropped() {
        let u = validate("https://e.com/p?a=1&b=2#section").unwrap();
        assert_eq!(u.query(), Some("a=1&b=2"));
        assert_eq!(u.fragment(), None);
    }

    #[test]
    fn non_http_schemes_are_refused() {
        for raw in [
            "file:///etc/passwd",
            "ftp://example.com",
            "javascript:alert(1)",
            "data:text/html,<h1>x</h1>",
            "gopher://example.com",
        ] {
            assert!(
                matches!(rejected(raw), UrlRejection::UnsupportedScheme(_)),
                "{raw} should be refused"
            );
        }
    }

    #[test]
    fn a_relative_url_is_refused() {
        assert_eq!(rejected("/just/a/path"), UrlRejection::NotAbsolute);
        assert_eq!(rejected("example.com"), UrlRejection::NotAbsolute);
    }

    // user:pass@host both sends credentials nobody granted and disguises the
    // real host from anyone reading the URL.
    #[test]
    fn embedded_credentials_are_refused() {
        assert_eq!(
            rejected("https://user:pass@example.com"),
            UrlRejection::EmbeddedCredentials
        );
        assert_eq!(
            rejected("https://user@example.com"),
            UrlRejection::EmbeddedCredentials
        );
    }

    #[test]
    fn loopback_and_private_v4_are_refused() {
        for raw in [
            "http://127.0.0.1/",
            "http://127.1.2.3/",
            "http://10.0.0.5/",
            "http://172.16.0.1/",
            "http://172.31.255.255/",
            "http://192.168.1.1/",
            "http://0.0.0.0/",
        ] {
            assert!(
                matches!(rejected(raw), UrlRejection::PrivateAddress(_)),
                "{raw} should be refused"
            );
        }
    }

    // The single most valuable address to an attacker on a cloud host.
    #[test]
    fn the_cloud_metadata_address_is_refused() {
        assert!(matches!(
            rejected("http://169.254.169.254/latest/meta-data/"),
            UrlRejection::PrivateAddress(_)
        ));
    }

    #[test]
    fn carrier_grade_nat_and_other_reserved_v4_are_refused() {
        for raw in [
            "http://100.64.0.1/", // CGNAT
            "http://192.0.0.1/",  // IETF protocol assignments
            "http://198.18.0.1/", // benchmarking
            "http://240.0.0.1/",  // reserved
            "http://255.255.255.255/",
        ] {
            assert!(
                matches!(rejected(raw), UrlRejection::PrivateAddress(_)),
                "{raw} should be refused"
            );
        }
    }

    #[test]
    fn public_v4_neighbours_of_blocked_ranges_are_allowed() {
        // 172.32 is public even though 172.16/12 is not; an over-broad check
        // would silently break real sites.
        for raw in [
            "http://172.32.0.1/",
            "http://9.255.255.255/",
            "http://11.0.0.1/",
            "http://100.63.255.255/",
            "http://100.128.0.1/",
            "http://8.8.8.8/",
        ] {
            assert!(validate(raw).is_ok(), "{raw} should be allowed");
        }
    }

    #[test]
    fn loopback_and_private_v6_are_refused() {
        for raw in [
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[fd12:3456::1]/",
            "http://[fe80::1]/",
            "http://[::]/",
            "http://[2001:db8::1]/",
        ] {
            assert!(
                matches!(rejected(raw), UrlRejection::PrivateAddress(_)),
                "{raw} should be refused"
            );
        }
    }

    // ::ffff:127.0.0.1 is loopback wearing a v6 costume.
    #[test]
    fn ipv4_mapped_v6_loopback_is_refused() {
        assert!(matches!(
            rejected("http://[::ffff:127.0.0.1]/"),
            UrlRejection::PrivateAddress(_)
        ));
        assert!(matches!(
            rejected("http://[::ffff:10.0.0.1]/"),
            UrlRejection::PrivateAddress(_)
        ));
    }

    #[test]
    fn public_v6_is_allowed() {
        assert!(validate("http://[2606:4700::1111]/").is_ok());
    }

    // A public URL that redirects to 127.0.0.1 is the standard way to turn an
    // allowed fetch into an internal one, so every hop is re-checked.
    #[test]
    fn redirect_hops_are_validated_the_same_way() {
        let bad = url::Url::parse("http://169.254.169.254/").unwrap();
        assert!(validate_hop(&bad).is_err());
        let good = url::Url::parse("https://example.com/next").unwrap();
        assert!(validate_hop(&good).is_ok());
    }

    #[test]
    fn hostnames_are_left_to_the_tunnel() {
        // Resolution happens at the proxy, so a name is not judged here; the
        // network it resolves on is the boundary. Documented in the module.
        assert!(validate("http://localhost/").is_ok());
        assert!(validate("http://internal.corp/").is_ok());
    }

    #[test]
    fn rejections_explain_themselves() {
        assert!(rejected("file:///x").to_string().contains("only http"));
        assert!(rejected("http://127.0.0.1/")
            .to_string()
            .contains("private or reserved"));
    }
}

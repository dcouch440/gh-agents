#[cfg(test)]
mod tests {
    use super::super::*;

    fn vpn(proxy: Option<&str>) -> EgressConfig {
        EgressConfig {
            mode: EgressMode::Vpn,
            proxy_url: proxy.map(str::to_string),
            is_production: false,
        }
    }

    const T: Duration = Duration::from_secs(5);

    // The whole point of the module: no proxy means no request, not a
    // silent direct connection.
    #[test]
    fn vpn_mode_without_a_proxy_refuses_rather_than_falling_back() {
        let err = client_from(&vpn(None), T).unwrap_err();
        assert!(matches!(err, EgressError::NotConfigured), "{err:?}");
    }

    #[test]
    fn vpn_mode_with_a_proxy_builds_a_client() {
        assert!(client_from(&vpn(Some("http://127.0.0.1:3128")), T).is_ok());
    }

    #[test]
    fn an_unparseable_proxy_url_refuses() {
        let err = client_from(&vpn(Some("not a url")), T).unwrap_err();
        assert!(matches!(err, EgressError::ClientBuild(_)), "{err:?}");
    }

    #[test]
    fn direct_mode_is_refused_in_production() {
        let cfg = EgressConfig {
            mode: EgressMode::Direct,
            proxy_url: None,
            is_production: true,
        };
        let err = client_from(&cfg, T).unwrap_err();
        assert!(
            matches!(err, EgressError::DirectRefusedInProduction),
            "{err:?}"
        );
    }

    #[test]
    fn direct_mode_is_allowed_outside_production() {
        let cfg = EgressConfig {
            mode: EgressMode::Direct,
            proxy_url: None,
            is_production: false,
        };
        assert!(client_from(&cfg, T).is_ok());
    }

    // An operator typo must not silently disable the tunnel.
    #[test]
    fn an_unrecognised_mode_parses_as_vpn() {
        for raw in [None, Some(""), Some("nonsense"), Some("VPN"), Some("off")] {
            assert_eq!(EgressMode::parse(raw), EgressMode::Vpn, "{raw:?}");
        }
    }

    #[test]
    fn direct_is_opt_in_and_case_insensitive() {
        assert_eq!(EgressMode::parse(Some("direct")), EgressMode::Direct);
        assert_eq!(EgressMode::parse(Some("  Direct ")), EgressMode::Direct);
    }

    #[test]
    fn the_default_mode_is_vpn() {
        assert_eq!(EgressMode::default(), EgressMode::Vpn);
    }

    // Without install() the module must behave as if nothing is configured,
    // so a code path that forgets to initialize fails closed.
    #[test]
    fn an_uninitialised_process_refuses_egress() {
        // `client` reads process-global state; in a test binary that never
        // calls install(), the fallback must be the refusing configuration.
        let err = client(T);
        assert!(
            err.is_err(),
            "uninitialised egress must refuse, got a client"
        );
    }
}

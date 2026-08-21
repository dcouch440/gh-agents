#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::http::{header::AUTHORIZATION, Request};
    use tower_governor::key_extractor::KeyExtractor;

    use crate::server::rate_limit::{RateLimitKey, SessionOrIpKeyExtractor};

    fn with_peer(req: Request<()>, ip: &str) -> Request<()> {
        let mut req = req;
        let addr = SocketAddr::new(ip.parse::<IpAddr>().unwrap(), 4000);
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(addr));
        req
    }

    fn bearer(token: &str) -> Request<()> {
        Request::builder()
            .uri("/api/workflows")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(())
            .unwrap()
    }

    #[test]
    fn keys_by_token_when_present() {
        let key = SessionOrIpKeyExtractor.extract(&bearer("token-a")).unwrap();
        assert!(matches!(key, RateLimitKey::Session(_)));
    }

    #[test]
    fn same_token_is_the_same_bucket() {
        let a = SessionOrIpKeyExtractor.extract(&bearer("token-a")).unwrap();
        let b = SessionOrIpKeyExtractor.extract(&bearer("token-a")).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_tokens_are_different_buckets() {
        let a = SessionOrIpKeyExtractor.extract(&bearer("token-a")).unwrap();
        let b = SessionOrIpKeyExtractor.extract(&bearer("token-b")).unwrap();
        assert_ne!(a, b);
    }

    /// The regression this module exists for: two users behind one proxy address
    /// must not share a bucket.
    #[test]
    fn same_peer_ip_different_tokens_do_not_collide() {
        let a = SessionOrIpKeyExtractor
            .extract(&with_peer(bearer("user-a"), "172.17.0.1"))
            .unwrap();
        let b = SessionOrIpKeyExtractor
            .extract(&with_peer(bearer("user-b"), "172.17.0.1"))
            .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn falls_back_to_ip_without_a_token() {
        let req = with_peer(
            Request::builder().uri("/api/health").body(()).unwrap(),
            "10.1.2.3",
        );
        let key = SessionOrIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))
        );
    }

    /// EventSource cannot set headers, so the query parameter is a real auth path
    /// and has to be keyed the same way the header is.
    #[test]
    fn reads_the_token_query_parameter() {
        let req = with_peer(
            Request::builder()
                .uri("/api/stream?token=query-token")
                .body(())
                .unwrap(),
            "10.1.2.3",
        );
        let key = SessionOrIpKeyExtractor.extract(&req).unwrap();
        assert!(matches!(key, RateLimitKey::Session(_)));
    }

    #[test]
    fn header_and_query_token_agree() {
        let from_header = SessionOrIpKeyExtractor.extract(&bearer("same")).unwrap();
        let from_query = SessionOrIpKeyExtractor
            .extract(
                &Request::builder()
                    .uri("/api/stream?token=same")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(from_header, from_query);
    }

    /// An empty or malformed header must not put every such caller in one bucket
    /// keyed on the empty string — fall through to the IP instead.
    #[test]
    fn empty_bearer_falls_back_to_ip() {
        let req = with_peer(
            Request::builder()
                .uri("/api/workflows")
                .header(AUTHORIZATION, "Bearer ")
                .body(())
                .unwrap(),
            "10.1.2.3",
        );
        let key = SessionOrIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))
        );
    }

    #[test]
    fn forwarded_for_beats_peer_ip_when_unauthenticated() {
        let req = with_peer(
            Request::builder()
                .uri("/api/health")
                .header("x-forwarded-for", "203.0.113.7")
                .body(())
                .unwrap(),
            "172.17.0.1",
        );
        let key = SessionOrIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)))
        );
    }
}

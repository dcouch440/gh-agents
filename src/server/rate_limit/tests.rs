#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::http::{header::AUTHORIZATION, Request};
    use tower_governor::key_extractor::KeyExtractor;
    use uuid::Uuid;

    use crate::server::auth::create_token;
    use crate::server::rate_limit::{RateLimitKey, SessionOrIpKeyExtractor};
    use crate::types::UserId;

    const SECRET: &[u8] = b"test-jwt-secret-for-rate-limit-keys";

    fn extractor() -> SessionOrIpKeyExtractor {
        SessionOrIpKeyExtractor::new(SECRET)
    }

    /// A token this server would actually accept. Anything else must not earn a
    /// session bucket, so tests that want one have to sign it properly.
    fn token_for(email: &str) -> String {
        create_token(SECRET, 1, UserId(Uuid::new_v4()), email, false).unwrap()
    }

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
        let key = extractor().extract(&bearer(&token_for("a@x.com"))).unwrap();
        assert!(matches!(key, RateLimitKey::Session(_)));
    }

    #[test]
    fn same_token_is_the_same_bucket() {
        let token = token_for("a@x.com");
        let a = extractor().extract(&bearer(&token)).unwrap();
        let b = extractor().extract(&bearer(&token)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_tokens_are_different_buckets() {
        let a = extractor().extract(&bearer(&token_for("a@x.com"))).unwrap();
        let b = extractor().extract(&bearer(&token_for("b@x.com"))).unwrap();
        assert_ne!(a, b);
    }

    /// The regression this module exists for: two users behind one proxy address
    /// must not share a bucket.
    #[test]
    fn same_peer_ip_different_tokens_do_not_collide() {
        let a = extractor()
            .extract(&with_peer(bearer(&token_for("a@x.com")), "172.17.0.1"))
            .unwrap();
        let b = extractor()
            .extract(&with_peer(bearer(&token_for("b@x.com")), "172.17.0.1"))
            .unwrap();
        assert_ne!(a, b);
    }

    /// The bypass this guard exists for. This layer runs before the auth
    /// extractor, so an unverified bearer string taken at face value would let a
    /// caller send a fresh random token per request, land in a fresh empty bucket
    /// every time, and never be rate limited at all. Junk must key by IP.
    #[test]
    fn unverifiable_token_falls_back_to_ip() {
        let key = extractor()
            .extract(&with_peer(bearer("not-a-real-jwt"), "10.1.2.3"))
            .unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))
        );
    }

    /// The same attack spelled out: many distinct forged tokens from one source
    /// must all land in the *same* bucket, or the limit means nothing.
    #[test]
    fn rotating_forged_tokens_share_one_ip_bucket() {
        let ex = extractor();
        let keys: Vec<_> = (0..5)
            .map(|i| {
                ex.extract(&with_peer(bearer(&format!("forged-{i}")), "10.1.2.3"))
                    .unwrap()
            })
            .collect();

        let expected = RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(keys.iter().all(|k| *k == expected), "got {keys:?}");
    }

    /// A token signed with someone else's secret is a forgery, not a session.
    #[test]
    fn token_signed_with_another_secret_falls_back_to_ip() {
        let foreign = create_token(
            b"a-different-secret",
            1,
            UserId(Uuid::new_v4()),
            "a@x.com",
            false,
        )
        .unwrap();

        let key = extractor()
            .extract(&with_peer(bearer(&foreign), "10.1.2.3"))
            .unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))
        );
    }

    #[test]
    fn falls_back_to_ip_without_a_token() {
        let req = with_peer(
            Request::builder().uri("/api/health").body(()).unwrap(),
            "10.1.2.3",
        );
        let key = extractor().extract(&req).unwrap();
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
                .uri(format!("/api/stream?token={}", token_for("a@x.com")))
                .body(())
                .unwrap(),
            "10.1.2.3",
        );
        let key = extractor().extract(&req).unwrap();
        assert!(matches!(key, RateLimitKey::Session(_)));
    }

    #[test]
    fn header_and_query_token_agree() {
        let token = token_for("a@x.com");
        let from_header = extractor().extract(&bearer(&token)).unwrap();
        let from_query = extractor()
            .extract(
                &Request::builder()
                    .uri(format!("/api/stream?token={token}"))
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
        let key = extractor().extract(&req).unwrap();
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
        let key = extractor().extract(&req).unwrap();
        assert_eq!(
            key,
            RateLimitKey::Ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)))
        );
    }

    /// The secret must not be reachable through a log line.
    #[test]
    fn debug_does_not_leak_the_secret() {
        let rendered = format!("{:?}", extractor());
        assert!(!rendered.contains("test-jwt-secret"), "{rendered}");
    }
}

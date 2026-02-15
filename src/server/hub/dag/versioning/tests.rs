#[cfg(test)]
mod tests {
    use crate::server::hub::dag::versioning::compute_content_hash;

    #[test]
    fn hash_is_deterministic() {
        let content = "Hello, world!";
        let hash1 = compute_content_hash(content);
        let hash2 = compute_content_hash(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn different_content_produces_different_hash() {
        let hash1 = compute_content_hash("Hello, world!");
        let hash2 = compute_content_hash("Goodbye, world!");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn hash_is_lowercase_hex() {
        let hash = compute_content_hash("test");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash, hash.to_lowercase());
    }

    #[test]
    fn empty_content_has_valid_hash() {
        let hash = compute_content_hash("");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn known_sha256_value() {
        // SHA-256 of "test" is well-known
        let hash = compute_content_hash("test");
        assert_eq!(
            hash,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
}

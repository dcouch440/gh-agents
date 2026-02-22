#[cfg(test)]
mod tests {
    use crate::server::tools::documents::title_to_ref_tag;

    // =========================================================================
    // title_to_ref_tag
    // =========================================================================

    #[test]
    fn title_to_ref_tag_basic() {
        assert_eq!(title_to_ref_tag("My Cool Document"), "my-cool-document");
    }

    #[test]
    fn title_to_ref_tag_special_chars() {
        assert_eq!(
            title_to_ref_tag("API Design (v2) — Draft!"),
            "api-design-v2--draft"
        );
    }

    #[test]
    fn title_to_ref_tag_empty() {
        assert_eq!(title_to_ref_tag(""), "");
    }

    #[test]
    fn title_to_ref_tag_single_word() {
        assert_eq!(title_to_ref_tag("README"), "readme");
    }
}

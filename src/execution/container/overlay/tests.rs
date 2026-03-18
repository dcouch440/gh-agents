#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use super::super::{
        is_whiteout_file, parse_inventory, parse_inventory_line, strip_whiteout_prefix,
    };

    // ── Whiteout Detection ────────────────────────────────────────────────

    #[test]
    fn whiteout_file_detected() {
        assert!(is_whiteout_file(".wh.foo.txt"));
        assert!(is_whiteout_file(".wh.dir"));
        assert!(is_whiteout_file(".wh..hidden"));
    }

    #[test]
    fn opaque_dir_marker_is_not_whiteout() {
        assert!(!is_whiteout_file(".wh..wh..opq"));
    }

    #[test]
    fn normal_files_are_not_whiteout() {
        assert!(!is_whiteout_file("main.py"));
        assert!(!is_whiteout_file(".gitignore"));
        assert!(!is_whiteout_file(""));
    }

    // ── Whiteout Prefix Stripping ─────────────────────────────────────────

    #[test]
    fn strip_prefix_from_whiteout() {
        assert_eq!(strip_whiteout_prefix(".wh.foo.txt"), "foo.txt");
        assert_eq!(strip_whiteout_prefix(".wh.dir"), "dir");
    }

    #[test]
    fn strip_prefix_from_non_whiteout_is_noop() {
        assert_eq!(strip_whiteout_prefix("normal.txt"), "normal.txt");
    }

    // ── Inventory Line Parsing ────────────────────────────────────────────

    #[test]
    fn parse_valid_inventory_line() {
        let entry = parse_inventory_line("src/main.py\tf\t1024").unwrap();
        assert_eq!(entry.path, PathBuf::from("src/main.py"));
        assert_eq!(entry.file_type, 'f');
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn parse_directory_entry() {
        let entry = parse_inventory_line("src\td\t4096").unwrap();
        assert_eq!(entry.path, PathBuf::from("src"));
        assert_eq!(entry.file_type, 'd');
    }

    #[test]
    fn parse_char_device_whiteout() {
        let entry = parse_inventory_line("old_file.txt\tc\t0").unwrap();
        assert_eq!(entry.path, PathBuf::from("old_file.txt"));
        assert_eq!(entry.file_type, 'c');
    }

    #[test]
    fn parse_invalid_line_returns_none() {
        assert!(parse_inventory_line("garbage").is_none());
        assert!(parse_inventory_line("").is_none());
        assert!(parse_inventory_line("\t\t").is_none());
    }

    #[test]
    fn parse_empty_path_returns_none() {
        assert!(parse_inventory_line("\tf\t100").is_none());
    }

    // ── Full Inventory Parsing ────────────────────────────────────────────

    #[test]
    fn parse_multi_line_inventory() {
        let output = "src/main.py\tf\t1024\nsrc\td\t4096\nREADME.md\tf\t256\n";
        let entries = parse_inventory(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, PathBuf::from("src/main.py"));
        assert_eq!(entries[1].file_type, 'd');
        assert_eq!(entries[2].path, PathBuf::from("README.md"));
    }

    #[test]
    fn parse_empty_inventory() {
        let entries = parse_inventory("");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_inventory_skips_invalid_lines() {
        let output = "valid.py\tf\t100\nbadline\nalso_valid.rs\tf\t200\n";
        let entries = parse_inventory(output);
        assert_eq!(entries.len(), 2);
    }

    // ── Classification Logic ──────────────────────────────────────────────
    // These test the classification rules that extract_overlay_diff applies.

    #[test]
    fn new_file_not_in_base() {
        let base: HashSet<PathBuf> = HashSet::new();
        let path = PathBuf::from("new_file.py");
        let is_modified = base.contains(&path);
        assert!(!is_modified); // Should be Created
    }

    #[test]
    fn modified_file_in_base() {
        let mut base: HashSet<PathBuf> = HashSet::new();
        base.insert(PathBuf::from("existing.py"));
        let path = PathBuf::from("existing.py");
        let is_modified = base.contains(&path);
        assert!(is_modified); // Should be Modified
    }

    #[test]
    fn whiteout_produces_deleted_for_correct_path() {
        // .wh.foo.txt in dir/ should produce Deleted for dir/foo.txt
        let whiteout_name = ".wh.foo.txt";
        assert!(is_whiteout_file(whiteout_name));
        let real_name = strip_whiteout_prefix(whiteout_name);
        assert_eq!(real_name, "foo.txt");

        let whiteout_path = PathBuf::from("dir/.wh.foo.txt");
        let real_path = whiteout_path
            .parent()
            .map(|p| {
                p.join(strip_whiteout_prefix(
                    whiteout_path.file_name().unwrap().to_str().unwrap(),
                ))
            })
            .unwrap();
        assert_eq!(real_path, PathBuf::from("dir/foo.txt"));
    }

    #[test]
    fn nested_whiteout_path() {
        // deep/nested/.wh.secret.key → Deleted for deep/nested/secret.key
        let path = PathBuf::from("deep/nested/.wh.secret.key");
        let file_name = path.file_name().unwrap().to_str().unwrap();
        assert!(is_whiteout_file(file_name));
        let real_name = strip_whiteout_prefix(file_name);
        let real_path = path.parent().unwrap().join(real_name);
        assert_eq!(real_path, PathBuf::from("deep/nested/secret.key"));
    }
}

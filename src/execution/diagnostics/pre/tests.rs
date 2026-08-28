#[cfg(test)]
mod tests {
    use crate::execution::diagnostics::envelope::{DiagnosticCategory, Severity};
    use crate::execution::diagnostics::pre::PreCheck;

    // ── State Persistence ─────────────────────────────────────────────────

    use crate::execution::diagnostics::pre::state_persistence::StatePersistenceCheck;

    #[test]
    fn standalone_cd_warns() {
        let check = StatePersistenceCheck;
        let d = check.check("cd /workspace/src").unwrap();
        assert_eq!(d.category, DiagnosticCategory::StatePersistence);
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn chained_cd_is_fine() {
        let check = StatePersistenceCheck;
        assert!(check.check("cd /workspace/src && make").is_none());
    }

    #[test]
    fn cd_with_semicolon_is_fine() {
        let check = StatePersistenceCheck;
        assert!(check.check("cd /workspace/src; make").is_none());
    }

    #[test]
    fn standalone_export_warns() {
        let check = StatePersistenceCheck;
        let d = check
            .check("export DATABASE_URL=postgres://localhost/db")
            .unwrap();
        assert_eq!(d.category, DiagnosticCategory::StatePersistence);
    }

    #[test]
    fn export_chained_is_fine() {
        let check = StatePersistenceCheck;
        assert!(check.check("export FOO=bar && python main.py").is_none());
    }

    #[test]
    fn standalone_alias_warns() {
        let check = StatePersistenceCheck;
        assert!(check.check("alias ll='ls -la'").is_some());
    }

    #[test]
    fn standalone_source_warns() {
        let check = StatePersistenceCheck;
        let d = check.check("source .env").unwrap();
        assert_eq!(d.severity, Severity::Info);
    }

    #[test]
    fn source_chained_is_fine() {
        let check = StatePersistenceCheck;
        assert!(check.check("source .env && python main.py").is_none());
    }

    #[test]
    fn dot_source_warns() {
        let check = StatePersistenceCheck;
        let d = check.check(". .env").unwrap();
        assert_eq!(d.severity, Severity::Info);
    }

    #[test]
    fn non_cd_export_is_fine() {
        let check = StatePersistenceCheck;
        assert!(check.check("python main.py").is_none());
        assert!(check.check("ls -la").is_none());
        assert!(check.check("make build").is_none());
    }

    #[test]
    fn chain_operator_in_quotes_ignored() {
        let check = StatePersistenceCheck;
        // The && is inside quotes — this is still a standalone cd
        let d = check.check("cd '/path/with && in name'").unwrap();
        assert_eq!(d.category, DiagnosticCategory::StatePersistence);
    }

    // ── Interactive Commands ──────────────────────────────────────────────

    use crate::execution::diagnostics::pre::interactive::InteractiveCheck;

    #[test]
    fn bare_python_warns() {
        let check = InteractiveCheck;
        let d = check.check("python").unwrap();
        assert_eq!(d.category, DiagnosticCategory::InteractiveCommand);
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn bare_python3_warns() {
        let check = InteractiveCheck;
        assert!(check.check("python3").is_some());
    }

    #[test]
    fn python_with_script_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("python main.py").is_none());
        assert!(check.check("python -c 'print(1)'").is_none());
        assert!(check.check("python3 script.py").is_none());
    }

    #[test]
    fn bare_node_warns() {
        let check = InteractiveCheck;
        assert!(check.check("node").is_some());
    }

    #[test]
    fn node_with_script_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("node app.js").is_none());
        assert!(check.check("node -e 'console.log(1)'").is_none());
    }

    #[test]
    fn mysql_without_e_warns() {
        let check = InteractiveCheck;
        let d = check.check("mysql -u root -p mydb").unwrap();
        assert_eq!(d.category, DiagnosticCategory::InteractiveCommand);
    }

    #[test]
    fn mysql_with_e_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("mysql -u root -e 'SELECT 1'").is_none());
    }

    #[test]
    fn psql_without_c_warns() {
        let check = InteractiveCheck;
        assert!(check.check("psql -U postgres mydb").is_some());
    }

    #[test]
    fn psql_with_c_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("psql -U postgres -c 'SELECT 1'").is_none());
    }

    #[test]
    fn psql_with_f_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("psql -U postgres -f schema.sql").is_none());
    }

    #[test]
    fn ssh_warns() {
        let check = InteractiveCheck;
        assert!(check.check("ssh user@host").is_some());
    }

    #[test]
    fn apt_get_without_y_warns() {
        let check = InteractiveCheck;
        let d = check.check("apt-get install curl").unwrap();
        assert_eq!(d.severity, Severity::Info);
    }

    #[test]
    fn apt_get_with_y_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("apt-get install -y curl").is_none());
    }

    #[test]
    fn normal_commands_are_fine() {
        let check = InteractiveCheck;
        assert!(check.check("ls -la").is_none());
        assert!(check.check("cat file.txt").is_none());
        assert!(check.check("pip install requests").is_none());
    }

    #[test]
    fn mysql_with_compact_e_flag_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("mysql -e'SELECT 1'").is_none());
        assert!(check.check("mysql -u root -e\"SELECT 1\"").is_none());
    }

    #[test]
    fn psql_with_compact_c_flag_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("psql -c'SELECT 1'").is_none());
        assert!(check.check("psql -c\"SELECT 1\"").is_none());
    }

    #[test]
    fn timeout_wrapping_bare_python_warns() {
        let check = InteractiveCheck;
        let d = check.check("timeout 30 python").unwrap();
        assert_eq!(d.category, DiagnosticCategory::InteractiveCommand);
    }

    #[test]
    fn sudo_wrapping_bare_node_warns() {
        let check = InteractiveCheck;
        let d = check.check("sudo node").unwrap();
        assert_eq!(d.category, DiagnosticCategory::InteractiveCommand);
    }

    #[test]
    fn timeout_wrapping_python_script_is_fine() {
        let check = InteractiveCheck;
        assert!(check.check("timeout 30 python main.py").is_none());
    }

    #[test]
    fn env_wrapping_mysql_without_e_warns() {
        let check = InteractiveCheck;
        assert!(check.check("env MYSQL_PWD=secret mysql -u root").is_some());
    }

    // ── Shell Compatibility ───────────────────────────────────────────────

    use crate::execution::diagnostics::pre::shell_compat::ShellCompatCheck;

    #[test]
    fn double_bracket_warns() {
        let check = ShellCompatCheck;
        let d = check
            .check("if [[ -f config.json ]]; then echo exists; fi")
            .unwrap();
        assert_eq!(d.category, DiagnosticCategory::ShellCompat);
    }

    #[test]
    fn single_bracket_is_fine() {
        let check = ShellCompatCheck;
        assert!(check
            .check("if [ -f config.json ]; then echo exists; fi")
            .is_none());
    }

    #[test]
    fn source_handled_by_persistence_not_compat() {
        // source is now handled by StatePersistenceCheck only, to avoid duplicates
        let check = ShellCompatCheck;
        assert!(check.check("source ~/.bashrc").is_none());
    }

    #[test]
    fn dot_command_is_fine() {
        // The ShellCompatCheck doesn't warn on `. file` — that's POSIX compliant.
        // (StatePersistenceCheck handles the persistence aspect separately.)
        let check = ShellCompatCheck;
        assert!(check.check(". ~/.profile").is_none());
    }

    #[test]
    fn process_substitution_warns() {
        let check = ShellCompatCheck;
        assert!(check.check("diff <(sort a.txt) <(sort b.txt)").is_some());
    }

    #[test]
    fn bash_array_warns() {
        let check = ShellCompatCheck;
        assert!(check.check("files=(a.py b.py c.py)").is_some());
    }

    #[test]
    fn subshell_not_false_positive() {
        let check = ShellCompatCheck;
        // $(...) is POSIX — should not trigger bash array detection
        assert!(check.check("result=$(echo hello)").is_none());
    }

    #[test]
    fn nvm_use_warns() {
        let check = ShellCompatCheck;
        let d = check.check("nvm use 18").unwrap();
        assert_eq!(d.category, DiagnosticCategory::ShellCompat);
    }

    #[test]
    fn conda_activate_warns() {
        let check = ShellCompatCheck;
        assert!(check.check("conda activate myenv").is_some());
    }

    #[test]
    fn normal_commands_pass() {
        let check = ShellCompatCheck;
        assert!(check.check("grep -r 'pattern' .").is_none());
        assert!(check.check("find . -name '*.py'").is_none());
        assert!(check.check("make build && ./test.sh").is_none());
    }

    #[test]
    fn double_bracket_in_quotes_not_flagged() {
        let check = ShellCompatCheck;
        assert!(check.check("echo '[[ not real ]]'").is_none());
        assert!(check.check("echo \"[[ not real ]]\"").is_none());
    }

    #[test]
    fn escaped_quotes_handled() {
        let check = StatePersistenceCheck;
        // The && is real (not inside quotes), so this is chained, not standalone
        assert!(check
            .check(r#"cd "/path with \"quotes\"" && make"#)
            .is_none());
    }

    // ── Heredoc truncation ────────────────────────────────────────────────
    use crate::execution::diagnostics::pre::heredoc::{unterminated_heredocs, HeredocCheck};

    /// The design-spec agent's `cat > spec.md << 'EOF' …` was truncated at ~146
    /// of 816 lines. The shell ran the fragment, wrote a broken file, and
    /// reported success; the agent spent three rounds and 2m40s blaming the
    /// shell while the file sat corrupt on disk.
    #[test]
    fn an_unterminated_heredoc_is_detected() {
        let cmd = "cat > spec.md << 'EOF'\n# Design Spec\n## Colour tokens\n- primary:";
        assert_eq!(unterminated_heredocs(cmd), vec!["EOF".to_string()]);
    }

    #[test]
    fn a_closed_heredoc_is_clean() {
        let cmd = "cat > spec.md << 'EOF'\n# Design Spec\nEOF";
        assert!(unterminated_heredocs(cmd).is_empty());
    }

    /// `<<` is also a left shift. Quoted arithmetic must not be read as a
    /// heredoc, or every inline python/awk call becomes a false rejection.
    #[test]
    fn a_left_shift_inside_quotes_is_not_a_heredoc() {
        assert!(unterminated_heredocs(r#"python -c "print(1 << 2)""#).is_empty());
        assert!(unterminated_heredocs("awk '{ x = 1 << 2; print x }' data.txt").is_empty());
    }

    /// `<<<` is a here-string, not a here-document, and needs no terminator.
    #[test]
    fn a_herestring_is_not_a_heredoc() {
        assert!(unterminated_heredocs("jq . <<< '{\"a\":1}'").is_empty());
    }

    /// `<<-` strips leading tabs from the terminator line.
    #[test]
    fn a_tab_indented_terminator_closes_a_dash_heredoc() {
        assert!(unterminated_heredocs("cat > f <<-EOF\n\tbody\n\tEOF").is_empty());
    }

    /// Two heredocs in one call, the first closed and the second not — the
    /// "write multiple files in one call" shape the old run_command
    /// description taught.
    #[test]
    fn only_the_unclosed_of_two_heredocs_is_reported() {
        let cmd = "cat > a.json << 'EOF'\n{}\nEOF\ncat > b.json << 'EOF'\n{\"k\":";
        assert_eq!(unterminated_heredocs(cmd).len(), 1);
    }

    /// The check must block, not merely warn — the whole point is that the
    /// broken fragment never reaches the shell.
    #[test]
    fn the_heredoc_check_reports_error_severity_and_truncation_category() {
        let d = HeredocCheck
            .check("cat > f << 'EOF'\nbody")
            .expect("must fire");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.category, DiagnosticCategory::Truncation);
        assert!(d.suggestion.unwrap().contains("write_file"));
    }

    /// An ordinary command with no heredoc at all must not fire.
    #[test]
    fn a_plain_command_is_not_a_heredoc() {
        assert!(HeredocCheck
            .check("pytest tests/ && python main.py")
            .is_none());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::execution::diagnostics::types::ChangeType;
    use crate::execution::diagnostics::DiagnosticsEngine;

    /// `write_file` and `edit_file` never touch the shell, so the
    /// snapshot → exec → snapshot path in `DiagnosticsEngine::execute` never
    /// sees them. Without an explicit bridge, the moment agents stop writing
    /// through heredocs the passdown `files:` line goes empty and downstream
    /// agents lose the objective record of what landed.
    #[test]
    fn a_file_tool_write_reaches_the_produced_file_manifest() {
        let mut engine = DiagnosticsEngine::new();
        engine.record_file_write("nexor/index.html".into(), ChangeType::Created, 4096);

        let files = engine.produced_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("nexor/index.html"));
        assert_eq!(files[0].size, 4096);
    }

    /// The repeat-edit nudge fired in run dd27d008 while the QA agent was
    /// fighting whitespace with `sed -i`. It must keep firing once that same
    /// agent is using `edit_file` instead.
    #[test]
    fn repeated_file_tool_edits_still_trip_the_loop_detector() {
        let mut engine = DiagnosticsEngine::new();
        let mut last = None;
        for _ in 0..3 {
            last = Some(engine.record_file_write(
                "nexor/styles.css".into(),
                ChangeType::Modified,
                900,
            ));
        }
        assert!(
            last.expect("recorded at least once").should_render(),
            "3 edits to one file must warn"
        );
    }

    /// A Created write is not an edit — otherwise every agent that writes three
    /// files gets a spurious loop warning on its third deliverable.
    #[test]
    fn creating_three_different_files_is_not_a_loop() {
        let mut engine = DiagnosticsEngine::new();
        for p in ["a.html", "b.css", "c.js"] {
            let status = engine.record_file_write(p.into(), ChangeType::Created, 10);
            assert!(!status.should_render(), "{p} should not warn");
        }
    }

    /// Later writes update the recorded size so the passdown reports what is on
    /// disk now, not what the first write left.
    #[test]
    fn a_rewrite_updates_the_recorded_size() {
        let mut engine = DiagnosticsEngine::new();
        engine.record_file_write("spec.md".into(), ChangeType::Created, 100);
        engine.record_file_write("spec.md".into(), ChangeType::Modified, 8000);

        let files = engine.produced_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, 8000);
    }
}

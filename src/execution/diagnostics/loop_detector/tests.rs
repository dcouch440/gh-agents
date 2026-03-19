#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::execution::diagnostics::loop_detector::{LoopDetector, LoopStatus};
    use crate::execution::diagnostics::types::{ChangeType, FileChange};

    fn modified(path: &str) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            change_type: ChangeType::Modified,
            size: 100,
        }
    }

    fn created(path: &str) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            change_type: ChangeType::Created,
            size: 100,
        }
    }

    #[test]
    fn single_edit_is_clean() {
        let mut detector = LoopDetector::new();
        let status = detector.record(1, &[modified("main.py")]);
        assert!(matches!(status, LoopStatus::Clean));
    }

    #[test]
    fn two_edits_is_clean() {
        let mut detector = LoopDetector::new();
        detector.record(1, &[modified("main.py")]);
        let status = detector.record(2, &[modified("main.py")]);
        assert!(matches!(status, LoopStatus::Clean));
    }

    #[test]
    fn three_edits_is_info() {
        let mut detector = LoopDetector::new();
        detector.record(1, &[modified("main.py")]);
        detector.record(2, &[modified("main.py")]);
        let status = detector.record(3, &[modified("main.py")]);
        match &status {
            LoopStatus::Info {
                file, edit_count, ..
            } => {
                assert_eq!(file, &PathBuf::from("main.py"));
                assert_eq!(*edit_count, 3);
            }
            _ => panic!("Expected Info, got {:?}", status),
        }
    }

    #[test]
    fn five_edits_is_warning() {
        let mut detector = LoopDetector::new();
        for i in 1..=5 {
            let status = detector.record(i, &[modified("config.json")]);
            if i == 5 {
                match &status {
                    LoopStatus::Warning {
                        file, edit_count, ..
                    } => {
                        assert_eq!(file, &PathBuf::from("config.json"));
                        assert_eq!(*edit_count, 5);
                    }
                    _ => panic!("Expected Warning at edit 5, got {:?}", status),
                }
            }
        }
    }

    #[test]
    fn warning_message_contains_loop_detected() {
        let mut detector = LoopDetector::new();
        for i in 1..=5 {
            detector.record(i, &[modified("x.py")]);
        }
        let status = detector.record(6, &[modified("x.py")]);
        match status {
            LoopStatus::Warning { message, .. } => {
                assert!(message.contains("LOOP DETECTED"));
                assert!(message.contains("x.py"));
            }
            _ => panic!("Expected Warning"),
        }
    }

    #[test]
    fn different_files_tracked_independently() {
        let mut detector = LoopDetector::new();
        detector.record(1, &[modified("a.py")]);
        detector.record(2, &[modified("b.py")]);
        detector.record(3, &[modified("a.py")]);
        detector.record(4, &[modified("b.py")]);
        let status = detector.record(5, &[modified("a.py")]);
        // a.py has 3 edits → Info. b.py has 2 → Clean.
        assert!(matches!(status, LoopStatus::Info { .. }));
    }

    #[test]
    fn created_files_not_tracked() {
        let mut detector = LoopDetector::new();
        for i in 1..=5 {
            detector.record(i, &[created("new.py")]);
        }
        let status = detector.record(6, &[created("new.py")]);
        assert!(matches!(status, LoopStatus::Clean));
    }

    #[test]
    fn should_render() {
        assert!(!LoopStatus::Clean.should_render());
        assert!(LoopStatus::Info {
            file: PathBuf::from("x"),
            edit_count: 3,
            message: "test".to_string(),
        }
        .should_render());
    }
}

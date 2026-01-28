//! Self-correction and recovery prompts for handling failures.
//!
//! This module provides prompts for recovering from:
//! - Parse errors (output couldn't be parsed)
//! - Test failures (code doesn't work as expected)
//! - Review rejections (code needs revisions)
//! - Stuck loops (repeated failures)
//! - Conflicting requirements (ambiguous specs)

mod prompts;

pub use prompts::*;

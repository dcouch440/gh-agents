//! Three-way merge using `diffy` with conflict marker parsing.
//!
//! Wraps `diffy::merge` and parses its conflict marker output into
//! structured `ConflictHunk` values for LLM resolution.

use super::types::{ConflictHunk, MergeResult};

/// Perform a three-way merge of base, version A, and version B.
///
/// Returns `MergeResult::Clean` if no conflicts, or `MergeResult::Conflicts`
/// with parsed conflict hunks for LLM resolution.
pub fn three_way_merge(base: &str, version_a: &str, version_b: &str) -> MergeResult {
    match diffy::merge(base, version_a, version_b) {
        Ok(merged) => MergeResult::Clean(merged),
        Err(conflicted) => {
            let hunks = parse_conflict_markers(&conflicted, base);
            MergeResult::Conflicts { conflicted, hunks }
        }
    }
}

/// Perform N-way merge by sequential pairwise reduction.
///
/// Merges versions one at a time against an accumulating result.
/// The original base stays constant throughout — each subsequent version
/// is merged using `diff3(original_base, accumulated, next_version)`.
/// This correctly combines non-overlapping changes from all versions.
///
/// Steps are expected to be sorted by `display_order` for determinism.
/// Conflicts from intermediate merges are returned immediately for
/// LLM resolution before the next pairwise merge can proceed.
pub fn n_way_merge(base: &str, versions: &[&str]) -> MergeResult {
    if versions.is_empty() {
        return MergeResult::Clean(base.to_string());
    }
    if versions.len() == 1 {
        return MergeResult::Clean(versions[0].to_string());
    }

    // First pair: merge versions[0] and versions[1] against the base.
    let result = three_way_merge(base, versions[0], versions[1]);

    if versions.len() == 2 {
        return result;
    }

    // For >2 versions, fold each subsequent version against the
    // original base. The accumulated result carries changes from all
    // prior versions; the original base is the common ancestor for all.
    match result {
        MergeResult::Conflicts { .. } => result,
        MergeResult::Clean(accumulated) => {
            let mut current = accumulated;
            for version in &versions[2..] {
                match three_way_merge(base, &current, version) {
                    MergeResult::Clean(merged) => current = merged,
                    conflicts @ MergeResult::Conflicts { .. } => return conflicts,
                }
            }
            MergeResult::Clean(current)
        }
    }
}

// ── Conflict Marker Parsing ──────────────────────────────────────────────────

/// Parse diffy's conflict marker output into structured hunks.
///
/// diffy uses standard conflict markers:
/// ```text
/// <<<<<<< original
/// base content
/// ||||||| modified
/// version A content
/// =======
/// version B content
/// >>>>>>> original
/// ```
///
/// We extract the three regions from each conflict block.
fn parse_conflict_markers(conflicted: &str, base: &str) -> Vec<ConflictHunk> {
    let lines: Vec<&str> = conflicted.lines().collect();
    let base_lines: Vec<&str> = base.lines().collect();
    let mut hunks = Vec::new();
    let mut i = 0;
    let mut base_line_cursor = 0;

    while i < lines.len() {
        if lines[i].starts_with("<<<<<<<") {
            i += 1;

            // Collect base/original lines until ||||||| or =======
            let mut base_section = Vec::new();
            while i < lines.len()
                && !lines[i].starts_with("|||||||")
                && !lines[i].starts_with("=======")
            {
                base_section.push(lines[i]);
                i += 1;
            }

            // If ||||||| separator found, this is a diff3-style conflict
            let mut version_a = Vec::new();
            if i < lines.len() && lines[i].starts_with("|||||||") {
                // The section before ||||||| was version A (or base, depending on diffy format)
                // diffy format: <<<<<<< = base, ||||||| = version_a, ======= = version_b
                // Actually, diffy uses: <<<<<<< original (base lines), ||||||| modified (version_a lines)
                // Let me handle both formats
                i += 1;
                while i < lines.len() && !lines[i].starts_with("=======") {
                    version_a.push(lines[i]);
                    i += 1;
                }
            } else {
                // Two-way style: <<<<<<< has version A, ======= has version B
                version_a = base_section.clone();
                base_section.clear();
            }

            // Skip ======= line
            if i < lines.len() && lines[i].starts_with("=======") {
                i += 1;
            }

            // Collect version B lines until >>>>>>>
            let mut version_b = Vec::new();
            while i < lines.len() && !lines[i].starts_with(">>>>>>>") {
                version_b.push(lines[i]);
                i += 1;
            }

            // Estimate base line range by searching for the base content
            let base_range = estimate_base_range(&base_section, &base_lines, base_line_cursor);
            base_line_cursor = base_range.end;

            hunks.push(ConflictHunk {
                base_lines: base_section.join("\n"),
                version_a_lines: version_a.join("\n"),
                version_b_lines: version_b.join("\n"),
                base_line_range: base_range,
            });

            if i < lines.len() {
                i += 1; // Skip >>>>>>> line
            }
        } else {
            // Track position in base file for non-conflict lines
            if base_line_cursor < base_lines.len() && lines[i] == base_lines[base_line_cursor] {
                base_line_cursor += 1;
            }
            i += 1;
        }
    }

    hunks
}

/// Estimate where a conflict's base content maps to in the original base file.
fn estimate_base_range(
    base_section: &[&str],
    base_lines: &[&str],
    search_from: usize,
) -> std::ops::Range<usize> {
    if base_section.is_empty() {
        return search_from..search_from;
    }

    // Search forward from cursor for the first line of the base section
    for start in search_from..base_lines.len() {
        if base_lines[start] == base_section[0] {
            let end = (start + base_section.len()).min(base_lines.len());
            // Verify the rest matches
            let matches = base_section
                .iter()
                .zip(&base_lines[start..end])
                .all(|(a, b)| a == b);
            if matches {
                return start..end;
            }
        }
    }

    // Fallback: approximate range
    search_from..search_from + base_section.len()
}

/// Reassemble a file from the conflicted output by replacing conflict
/// marker blocks with their LLM-resolved content.
///
/// `resolved_hunks` must be in the same order as `hunks` in the
/// `MergeResult::Conflicts` variant.
pub fn reassemble(conflicted: &str, resolved_hunks: &[String]) -> String {
    let lines: Vec<&str> = conflicted.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut hunk_idx = 0;

    while i < lines.len() {
        if lines[i].starts_with("<<<<<<<") {
            // Skip the entire conflict block
            while i < lines.len() && !lines[i].starts_with(">>>>>>>") {
                i += 1;
            }
            if i < lines.len() {
                i += 1; // Skip >>>>>>> line
            }

            // Insert the resolved content
            if hunk_idx < resolved_hunks.len() {
                result.push(resolved_hunks[hunk_idx].as_str());
                hunk_idx += 1;
            }
        } else {
            result.push(lines[i]);
            i += 1;
        }
    }

    result.join("\n")
}

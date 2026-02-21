//! Render dispatch snapshots as structured XML.
//!
//! Uses `crate::markup::XmlBuilder` to produce `<dispatch_status>` blocks
//! with `<dispatch>` child elements.

use crate::markup::XmlBuilder;

use super::types::{DispatchSnapshot, DispatchStatus};

/// Render dispatch snapshots as `<dispatch_status>` XML.
///
/// Always returns a `<dispatch_status>` block so the assistant knows
/// the dispatch system exists even when nothing is in flight.
pub fn render(snapshots: &[DispatchSnapshot]) -> String {
    if snapshots.is_empty() {
        return "<dispatch_status>\nNo active dispatches.\n</dispatch_status>\n".to_string();
    }

    let mut root = XmlBuilder::new("dispatch_status", 0);

    for snap in snapshots {
        let mut el = XmlBuilder::new("dispatch", 1);
        el.attr("id", &snap.id);
        el.attr("instruction", &snap.instruction);
        el.attr("status", snap.status.as_str());

        // Time attribute: "started" for in-progress, "completed" for terminal
        let time_attr = if snap.status == DispatchStatus::InProgress {
            "started"
        } else {
            "completed"
        };
        el.attr(time_attr, &snap.elapsed);

        // Result attribute for terminal tasks
        if let Some(ref result) = snap.result {
            el.attr("result", result);
        }

        root.raw(&el.build());
    }

    root.build()
}

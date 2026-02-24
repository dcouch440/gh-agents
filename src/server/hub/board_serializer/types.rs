//! Board serializer types — Excalidraw input elements, canvas snapshots, and changesets.
//!
//! # Input types
//!
//! [`ExcalidrawElement`] is a tagged enum representing the subset of Excalidraw
//! element types we care about. Unknown element types collapse to `Other` and
//! are classified as noise.
//!
//! # Output types
//!
//! [`CanvasSnapshot`] is the structured representation of the board after
//! classification and annotation resolution. [`CanvasChangeset`] is the result
//! of diffing two snapshots.

use serde::Deserialize;

// ============================================================================
// Excalidraw Input Types
// ============================================================================

/// A single Excalidraw element, deserialized from the frontend's JSON payload.
///
/// We only model the element types relevant to board serialization. All other
/// types (ellipse, diamond, image, etc.) collapse to `Other`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ExcalidrawElement {
    #[serde(rename = "rectangle")]
    Rectangle(RectangleElement),
    #[serde(rename = "arrow")]
    Arrow(ArrowElement),
    #[serde(rename = "text")]
    Text(TextElement),
    #[serde(rename = "freedraw")]
    Freedraw(FreedrawElement),
    #[serde(rename = "line")]
    Line(LineElement),
    #[serde(other)]
    Other,
}

/// An Excalidraw rectangle element — potential node candidate if it has bound text.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectangleElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub is_deleted: bool,
    /// References to elements bound inside this rectangle (text labels, arrows).
    #[serde(default)]
    pub bound_elements: Vec<BoundElementRef>,
}

/// A reference to an element bound inside a shape.
#[derive(Debug, Clone, Deserialize)]
pub struct BoundElementRef {
    pub id: String,
    /// Element type — typically `"text"` or `"arrow"`.
    #[serde(rename = "type")]
    pub kind: String,
}

/// An Excalidraw arrow element — potential edge if both endpoints bind to node candidates.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub is_deleted: bool,
    /// Binding to the element at the arrow's start point.
    pub start_binding: Option<ArrowBinding>,
    /// Binding to the element at the arrow's end point.
    pub end_binding: Option<ArrowBinding>,
}

/// An arrow endpoint binding — references the element the arrow connects to.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowBinding {
    pub element_id: String,
}

/// An Excalidraw freedraw element — a freehand pen stroke stored as coordinate samples.
///
/// Points are relative offsets from the element's `(x, y)` origin.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreedrawElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub is_deleted: bool,
    /// Pen sample points, each `[dx, dy]` relative to `(x, y)`.
    #[serde(default)]
    pub points: Vec<Vec<f64>>,
}

/// An Excalidraw line element — a polyline stored as vertex coordinates.
///
/// Points are relative offsets from the element's `(x, y)` origin.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub is_deleted: bool,
    /// Vertex points, each `[dx, dy]` relative to `(x, y)`.
    #[serde(default)]
    pub points: Vec<Vec<f64>>,
}

/// An Excalidraw text element — either bound inside a shape (`container_id` is set)
/// or free-floating on the canvas.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub is_deleted: bool,
    /// The text content.
    #[serde(default)]
    pub text: String,
    /// If this text is bound inside a shape, this is the shape's element ID.
    /// `None` means the text is free-floating on the canvas.
    pub container_id: Option<String>,
}

// ============================================================================
// Canvas Snapshot Output Types
// ============================================================================

/// Structured snapshot of the Excalidraw board after classification and
/// annotation resolution. Contains nodes (rectangles with text), edges
/// (arrows between nodes), and global notes (text not near any node).
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasSnapshot {
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
    pub global_notes: Vec<GlobalNote>,
}

/// A node candidate: a rectangle with bound text on the canvas.
///
/// The `raw_text` contains the full box content — name, protocol hint,
/// instruction — as written by the user. The AI parses it downstream.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasNode {
    /// Excalidraw element ID (stable across sessions).
    pub element_id: String,
    /// Full text content of the box.
    pub raw_text: String,
    /// Bounding box on the canvas.
    pub bounds: CanvasBounds,
    /// Unbound text assigned to this node by spatial proximity.
    pub annotations: Vec<String>,
    /// ASCII rasterization of freeform drawings inside this node's bounds.
    /// `None` if no strokes were detected inside the node.
    pub sketch: Option<String>,
}

/// An edge: an arrow connecting two node candidates.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasEdge {
    /// Excalidraw arrow element ID.
    pub element_id: String,
    /// Element ID of the source rectangle (arrow start).
    pub source_node_id: String,
    /// Element ID of the target rectangle (arrow end).
    pub target_node_id: String,
}

/// Text on the board not near any node — board-level context for the AI.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalNote {
    /// Excalidraw text element ID.
    pub element_id: String,
    /// The text content.
    pub text: String,
}

/// Axis-aligned bounding box on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// ============================================================================
// Changeset Types (Snapshot Diff Output)
// ============================================================================

/// The result of diffing two [`CanvasSnapshot`]s. Categorizes every change
/// into one of the semantic diff categories from the Visual Dispatch vision:
///
/// - **New**: elements in current but not previous
/// - **Updated**: same element, text or annotations changed
/// - **Deleted**: elements in previous but not current
/// - **Moved**: same element, only position changed
/// - **Rewired**: same edge, different source or target
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasChangeset {
    /// Nodes that exist in current but not previous.
    pub new_nodes: Vec<CanvasNode>,
    /// Nodes whose text or annotations changed.
    pub updated_nodes: Vec<NodeUpdate>,
    /// Element IDs of nodes that were deleted.
    pub deleted_node_ids: Vec<String>,
    /// Nodes that moved but content didn't change.
    pub moved_nodes: Vec<NodeMove>,
    /// Edges that exist in current but not previous.
    pub new_edges: Vec<CanvasEdge>,
    /// Element IDs of edges that were deleted.
    pub deleted_edge_ids: Vec<String>,
    /// Edges that changed source or target (rewired).
    pub rewired_edges: Vec<EdgeRewire>,
}

/// A node whose text or annotations changed between snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeUpdate {
    pub element_id: String,
    pub old_text: String,
    pub new_text: String,
    pub old_annotations: Vec<String>,
    pub new_annotations: Vec<String>,
}

/// A node that moved (bounds changed) but content stayed the same.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeMove {
    pub element_id: String,
    pub old_bounds: CanvasBounds,
    pub new_bounds: CanvasBounds,
}

/// An edge that was rewired — same element ID but different endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRewire {
    pub element_id: String,
    pub old_source: String,
    pub old_target: String,
    pub new_source: String,
    pub new_target: String,
}

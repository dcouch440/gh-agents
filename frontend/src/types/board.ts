// ============================================================================
// Board Submit Types — mirrors backend response from POST /workflows/:id/board/submit
// ============================================================================

import type { WorkflowStep } from './workflow'

// ── Response ────────────────────────────────────────────────────────────────

/** Top-level response from `POST /workflows/:id/board/submit`. */
type BoardSubmitResponse = {
  readonly is_first_submit: boolean
  readonly changeset: FilteredChangeset
  readonly snapshot: CanvasSnapshot
  readonly phase_zero: PhaseZeroResponse
  readonly dispatch: BoardDispatchInfo | null
}

/**
 * Phase 0 structural execution result.
 *
 * `created_steps` and `updated_steps` contain full step objects (flattened)
 * with an `element_id` field for Excalidraw → step mapping. The frontend
 * uses these for selective `nmSet` into workflowStore — only new/changed
 * nodes cause re-renders.
 */
type PhaseZeroResponse = {
  readonly created_steps: readonly PhaseZeroStep[]
  readonly created_edges: readonly ElementEdgePair[]
  readonly deleted_steps: readonly string[]
  readonly deleted_edges: readonly string[]
  readonly rewired_edges: readonly ElementEdgePair[]
  readonly moved_steps: readonly string[]
  readonly updated_steps: readonly PhaseZeroStep[]
}

/**
 * A step created or updated by Phase 0, with the Excalidraw element ID
 * that produced it. The step fields are flattened (not nested under a `step` key).
 */
type PhaseZeroStep = WorkflowStep & {
  readonly element_id: string
}

type ElementEdgePair = {
  readonly element_id: string
  readonly edge_id: string
  readonly from_step_id: string
  readonly to_step_id: string
}

type BoardDispatchInfo = {
  readonly execution_id: string
  readonly session_id: string
  readonly step_id: string
  readonly instruction: string
}

// ── Canvas Snapshot ─────────────────────────────────────────────────────────

/** Structured representation of the Excalidraw board after classification. */
type CanvasSnapshot = {
  readonly nodes: readonly CanvasNode[]
  readonly edges: readonly CanvasEdge[]
  readonly global_notes: readonly GlobalNote[]
}

/** A node candidate: a rectangle with bound text on the canvas. */
type CanvasNode = {
  readonly element_id: string
  readonly raw_text: string
  readonly bounds: CanvasBounds
  readonly annotations: readonly string[]
  readonly sketch: string | null
  readonly stroke_encoding: string | null
}

/** An edge: an arrow connecting two node candidates. */
type CanvasEdge = {
  readonly element_id: string
  readonly source_node_id: string
  readonly target_node_id: string
}

/** Text on the board not near any node — board-level context. */
type GlobalNote = {
  readonly element_id: string
  readonly text: string
}

/** Axis-aligned bounding box on the canvas. */
type CanvasBounds = {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

// ── Changeset ───────────────────────────────────────────────────────────────

/**
 * Three-tier filtered changeset from the board serializer.
 *
 * - **Agentless**: structural changes handled as pure DB writes
 * - **Noise**: changes that survived diff but have no semantic meaning
 * - **Meaningful**: changes worth sending to an AI agent
 */
type FilteredChangeset = {
  readonly agentless: AgentlessChanges
  readonly noise: readonly FilteredNoise[]
  readonly meaningful: readonly ScoredChange[]
  readonly aggregate_score: number
  readonly should_dispatch: boolean
}

/** Changes applied as agentless DB writes (deletes, rewires, moves). */
type AgentlessChanges = {
  readonly deleted_node_ids: readonly string[]
  readonly deleted_edge_ids: readonly string[]
  readonly rewired_edges: readonly EdgeRewire[]
  readonly moved_nodes: readonly NodeMove[]
}

/** A meaningful change with significance score. */
type ScoredChange =
  | { readonly NewNode: { node: CanvasNode; significance: ChangeSignificance } }
  | { readonly UpdatedNode: { update: NodeUpdate; significance: ChangeSignificance; token_change_ratio: number } }
  | { readonly NewEdge: { edge: CanvasEdge; significance: ChangeSignificance } }

type ChangeSignificance = 'Low' | 'Medium' | 'High'

type NodeUpdate = {
  readonly element_id: string
  readonly old_text: string
  readonly new_text: string
  readonly old_annotations: readonly string[]
  readonly new_annotations: readonly string[]
}

type NodeMove = {
  readonly element_id: string
  readonly old_bounds: CanvasBounds
  readonly new_bounds: CanvasBounds
}

type EdgeRewire = {
  readonly element_id: string
  readonly old_source: string
  readonly old_target: string
  readonly new_source: string
  readonly new_target: string
}

type FilteredNoise = {
  readonly element_id: string
  readonly reason: NoiseReason
}

type NoiseReason = 'WhitespaceOnly' | 'Oscillation' | 'CanvasPan' | 'ReorderOnly'

// ── Board Elements (GET) ────────────────────────────────────────────────────

/** Response from `GET /workflows/:id/board/elements`. */
type BoardElementsResponse = {
  readonly elements: readonly Record<string, unknown>[] | null
  /** Last board submit response for debug panel rehydration on refresh. */
  readonly last_submit: BoardSubmitResponse | null
}

// ── Exports ─────────────────────────────────────────────────────────────────

export type {
  BoardElementsResponse,
  BoardSubmitResponse,
  PhaseZeroResponse,
  PhaseZeroStep,
  ElementEdgePair,
  BoardDispatchInfo,
  CanvasSnapshot,
  CanvasNode,
  CanvasEdge,
  GlobalNote,
  CanvasBounds,
  FilteredChangeset,
  AgentlessChanges,
  ScoredChange,
  ChangeSignificance,
  NodeUpdate,
  NodeMove,
  EdgeRewire,
  FilteredNoise,
  NoiseReason,
}

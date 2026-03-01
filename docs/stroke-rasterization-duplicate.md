# Stroke Rasterization — Duplicate Image Bug

## Problem

A single board submit with one node produces **two** PNG rasterizations of the same strokes. One looks correct; the other has missing geometry and degraded quality. The bad image is produced from a lossy intermediate format that strips pressure data and reduces point count.

## The Two Code Paths

### Path 1: Snapshot (correct)

**When:** Board submit (Phase 0)
**File:** `src/server/hub/board/serializer/snapshot.rs:95-107`

```
Raw Excalidraw freedraw elements
  → classify.rs: convert relative → absolute [x, y, pressure]
  → snapshot.rs: filter by node_id
  → rasterize_png::rasterize_strokes_png(node_strokes_3, ...)
  → stroke_png_base64 stored on CanvasNode
```

- Uses full `[x, y, pressure]` triples
- All original points preserved
- Result: `CanvasNode.stroke_png_base64`

### Path 2: Pipeline re-rasterization (broken)

**When:** Workflow execution (designer phase / single step execution)
**Files:**
- `src/server/hub/dag/pipeline/mod.rs:237` (workforce execution)
- `src/server/hub/dag/single/mod.rs:256` (single step execution)
- `src/server/hub/dag/pipeline/mod.rs:323-375` (the re-rasterizer)

```
Raw [x, y, pressure] triples
  → snapshot.rs:78-80: strip pressure → [x, y] pairs
  → encode.rs: RDP simplify (epsilon=3.0) → drops points
  → encode.rs: convert to node-relative integers → loses precision
  → stored as board_context_cache string:
    "## Stroke Coordinates\n{"canvas":[w,h],"strokes":[{"points":[[x,y],...]}]}"
  → (later, at execution time)
  → pipeline/mod.rs: parse JSON back out
  → pipeline/mod.rs: pressure defaults to 0.5 for all points
  → rasterize_strokes_png() → degraded PNG
```

Data losses in this path:
1. **Pressure stripped** — `[x, y, pressure]` → `[x, y]` at snapshot.rs:80
2. **Points removed** — RDP simplification with epsilon=3.0 can remove 60-80% of points
3. **Precision lost** — coordinates rounded to integers and made relative to node bounds
4. **Pressure fabricated** — re-rasterizer defaults all pressure to 0.5

## Where Each Path Is Called

| Call site | File | Line | What triggers it |
|-----------|------|------|-----------------|
| Snapshot PNG | `snapshot.rs` | 107 | `classify_board()` during board submit |
| Pipeline re-raster | `pipeline/mod.rs` | 237 | Workforce designer input at execution time |
| Single re-raster | `single/mod.rs` | 256 | Single-step execution |

## Fix Options

### Option A: Use the snapshot PNG at execution time (recommended)

The correct PNG already exists as `CanvasNode.stroke_png_base64` and gets stored in the DB (via `WorkflowStepRow.board_context_cache` or similar). At execution time, read the pre-rendered PNG instead of re-rasterizing from the lossy encoded format.

This eliminates the second code path entirely. The `rasterize_stroke_image_from_context()` function in `pipeline/mod.rs` becomes unnecessary.

### Option B: Store full stroke data in board_context_cache

Change `encode_strokes()` to preserve pressure and skip RDP simplification. This increases token cost in the context cache but produces identical rasterization.

### Option C: Store the base64 PNG directly in board_context_cache

Instead of storing encoded coordinates that get re-rasterized, store the already-rendered PNG base64 string. Zero data loss, no re-rasterization needed.

## Key Files

- `src/server/hub/board/serializer/snapshot.rs` — builds CanvasNode with both stroke_encoding and stroke_png_base64
- `src/server/hub/board/serializer/encode.rs` — RDP encoder that produces the lossy format
- `src/server/hub/dag/pipeline/mod.rs:323-375` — re-rasterizer that reads the lossy format
- `src/server/hub/dag/pipeline/mod.rs:237` — workforce call site
- `src/server/hub/dag/single/mod.rs:256` — single step call site
- `src/server/hub/board/serializer/rasterize_png.rs` — the actual PNG rasterizer (shared by both paths)
- `src/server/services/board/executor.rs:286-317` — builds board_context_cache string from stroke_encoding

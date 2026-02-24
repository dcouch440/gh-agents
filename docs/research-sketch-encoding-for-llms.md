# Research: Sketch Encoding for LLM Consumption

> Date: 2026-02-23
> Context: Board serializer rasterizes freehand strokes to a 48×24 ASCII grid (~1,200 tokens). Need a more token-efficient encoding that LLMs can actually understand.

## Problem

A 48×24 character grid using `█` (filled) and `·` (empty) costs ~1,200 tokens per sketch. Most cells are empty. Unicode characters like `█` (3 bytes UTF-8) tokenize poorly in BPE tokenizers — each gets its own token.

Grid dimensions (from `snapshot.rs:47-48`):
- `RASTER_COLS = 48`, `RASTER_ROWS = 24`
- Fixed regardless of input pixel size
- 500px wide box → ~10.4 px per cell resolution

## Key Finding: LLMs Are Bad at Grids, Good at Coordinates

### Paper: "From Text to Space" (Martorell, Feb 2025)
- **arxiv.org/abs/2502.16690**
- Tested six LLaMA-3 variants (1B-90B) on grid-world navigation
- **JSON Cartesian coordinates: 98% success** (90B model)
- **Grid/topographic layout: 30% success** (same model)
- 36-point advantage for coordinates over grids
- "LLMs prefer spatial info presented mathematically where x and y can be operated separately"

### Paper: ArtPrompt (Jiang et al., ACL 2024)
- **aclanthology.org/2024.acl-long.809**
- Vision-in-Text Challenge (ViTC) benchmark
- GPT-4 (best): **25% accuracy** on single-character ASCII art recognition
- GPT-4: **3.26% accuracy** on multi-character ASCII sequences
- BPE tokenization destroys spatial relationships

### Paper: "Stuck in the Matrix" (Oct 2025)
- **arxiv.org/abs/2510.20198**
- GPT-4o, GPT-4.1, Claude 3.7 Sonnet
- **42.7% average accuracy drop** as grid sizes increase (up to 84%)
- Claude 3.7 outperformed GPT on most tasks
- Removing spaces between grid chars improved tokenization but hurt some tasks

### Paper: ArtPerception (Yang et al., 2025)
- **arxiv.org/html/2510.10281v1**
- Extended ArtPrompt — advanced prompting techniques give only marginal improvement

## Encoding Comparison

| Encoding | Tokens | LLM Comprehension | Evidence |
|---|---|---|---|
| Raw 48×24 grid (`█`/`·`) | ~1,200 | Poor (25% accuracy) | ArtPrompt |
| RLE compressed grid | ~300-500 | Poor (still grid-based) | ArtPrompt reasoning applies |
| Braille/Unicode blocks | ~290-430 | Very poor (alien chars) | ArtPrompt + tokenizer analysis |
| Quadtree hierarchical | ~50-200 | Unknown/likely poor | No published research |
| Chain code (directions) | ~30-80 | Unknown but promising | Freeman 1961, untested with LLMs |
| **JSON coordinates** | **~30-100** | **Strong (98%)** | From Text to Space |
| **Natural language paths** | **~40-80** | **Excellent** | Native LLM format |
| **RDP-simplified strokes** | **~30-100** | **Strong** | Combines coordinate + simplification |

## Recommended: RDP-Simplified Stroke Paths

### Algorithm: Ramer-Douglas-Peucker (RDP)
- **en.wikipedia.org/wiki/Ramer-Douglas-Peucker_algorithm**
- Reduces polyline points while preserving shape
- A freehand stroke with 20 Bresenham points → 3-5 key vertices
- Epsilon parameter controls simplification aggressiveness

### Why This Fits Our Pipeline

We already have raw stroke coordinates in `classify.rs` (absolute points computed from Excalidraw freedraw/line elements). The current pipeline:

```
Excalidraw freedraw points (relative)
  → absolute coordinates (classify.rs)
    → Bresenham rasterization to 48×24 grid (rasterize.rs)
      → ASCII string stored in CanvasNode.sketch
```

The new encoder would branch off BEFORE rasterization:

```
Excalidraw freedraw points (relative)
  → absolute coordinates (classify.rs)
    → RDP simplification (new)
      → JSON stroke encoding (new) → ~30-100 tokens
    → ASCII grid (existing) → kept for debug/human viewing
```

### Output Format

```json
{
  "canvas": [480, 360],
  "strokes": [
    {"points": [[20,20],[460,20]]},
    {"points": [[130,70],[130,290]]},
    {"points": [[50,100],[150,100],[300,100]]}
  ]
}
```

Or natural language alternative:
```
Canvas: 480×360px. Strokes:
- horizontal line (20,20)→(460,20)
- vertical line (130,70)→(130,290)
- horizontal line (50,100)→(300,100)
```

### Implementation Plan

1. **New module**: `src/server/hub/board_serializer/encode.rs`
2. **RDP function**: `simplify_stroke(points: &[[f64; 2]], epsilon: f64) -> Vec<[f64; 2]>`
3. **Encoder function**: `encode_strokes_for_llm(strokes: &[ClassifiedStroke], bounds: &CanvasBounds) -> String`
4. **Add to CanvasNode**: New field `stroke_encoding: Option<String>` alongside existing `sketch: Option<String>`
5. **Tests**: Verify token reduction, verify LLM can interpret output

### RDP Algorithm (pseudocode)

```
function rdp(points, epsilon):
    if points.len() <= 2: return points

    // Find point furthest from line between first and last
    max_dist = 0
    max_idx = 0
    for i in 1..points.len()-1:
        dist = perpendicular_distance(points[i], points[0], points[last])
        if dist > max_dist:
            max_dist = dist
            max_idx = i

    if max_dist > epsilon:
        // Recurse on both halves
        left = rdp(points[0..=max_idx], epsilon)
        right = rdp(points[max_idx..], epsilon)
        return left[..last] + right
    else:
        // All points close to line — keep only endpoints
        return [points[0], points[last]]
```

## Additional Research References

### SVG-based approaches
- **VDLM** (Wang et al., EMNLP 2024) — arxiv.org/abs/2404.06479
  - LLMs cannot reliably interpret raw SVG zero-shot
  - "Primal Visual Description" (attributes as text) + GPT-4o: 76.9% accuracy
- **LLM SVG Understanding** — arxiv.org/html/2306.06094v2
  - GPT-4 with Chain-of-Thought on SVG spatial reasoning: 89% accuracy
- **SVGenius benchmark** — arxiv.org/html/2506.03139v1

### Sketch-RNN format (Google Brain 2017)
- **arxiv.org/abs/1704.03477** (Ha & Eck)
- 5-element per point: `(dx, dy, p1, p2, p3)` with pen state
- Delta encoding from previous point — very compact

### Token efficiency research
- **"Text or Pixels? It Takes Half"** — arxiv.org/abs/2510.18279
  - Rendering text as image for multimodal LLMs reduces decoder tokens ~50%
  - For multimodal models, sending a PNG could beat any text encoding
- **LLMLingua** — github.com/microsoft/LLMLingua
  - Up to 20x prompt compression, 1.5% performance loss

### Other
- **Chain codes** — Freeman 1961, direction sequences (0=E, 2=N, 4=W, 6=S)
- **Drawille** — github.com/asciimoo/drawille (braille pixel graphics)
- **Google Encoded Polyline** — compact but LLMs can't decode the Base64
- **Spatial Text Rendering** — medium.com/abwab-ai/spatial-text-rendering-pushing-spatial-understanding-of-llms-09d1a836bd66

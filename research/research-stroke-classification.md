# Research: Stroke Classification — Text vs. Shapes vs. Decorative Marks

> Date: 2026-02-23
> Context: Board serializer captures freehand strokes as coordinate arrays. Need to classify strokes into handwriting (→ Google Ink Recognition → plain text), shapes/diagrams (→ RDP → JSON coordinates), and decorative marks (→ label only) before sending to LLM.

## Problem

Users draw on a whiteboard canvas. Each stroke is a sequence of `(x, y)` points. The strokes could be:
1. **Handwriting** — text the user wrote by hand
2. **Shapes/diagrams** — boxes, arrows, wireframes, flowcharts
3. **Decorative marks** — underlines, squiggles, emphasis, doodles

Each category has a different optimal encoding for LLM consumption:
- Handwriting → Google Ink Recognition → plain text (~5 tokens)
- Shapes → RDP-simplified JSON coordinates (~30-100 tokens)
- Decorative → label only ("underline", "squiggle") (~2 tokens)

## Key Finding: Geometric Heuristics Get 80%+ Without ML

### Rubine's 13 Features (1991, foundational)

All features are incrementally computable in O(1) per input point. From Dean Rubine, "Specifying Gestures by Example" (SIGGRAPH 1991).

| # | Feature | What it measures |
|---|---------|-----------------|
| f1 | cos(initial angle) | Horizontal component of opening direction |
| f2 | sin(initial angle) | Vertical component of opening direction |
| f3 | Bounding box diagonal length | Overall spatial extent |
| f4 | Bounding box diagonal angle | Orientation of gesture envelope |
| f5 | Start-to-end distance | How "open" or "closed" the stroke is |
| f6 | cos(endpoint direction) | Horizontal component of overall direction |
| f7 | sin(endpoint direction) | Vertical component of overall direction |
| f8 | Total stroke path length | Total distance traversed |
| f9 | Total angle traversed | Cumulative angular change (captures winding) |
| f10 | Sum of absolute angles | Total turning magnitude regardless of direction |
| f11 | Sum of squared angles (sharpness) | Distinguishes smooth curves from sharp corners |
| f12 | Maximum speed | Peak velocity during execution |
| f13 | Total duration | Time from first to last point |

Note: f12 and f13 require timestamps. Our Excalidraw strokes don't have timestamps, so we'd use the first 11 features.

### Derived Composite Features

Computed from raw coordinates — no timestamps needed:

```
path_length       = sum of euclidean distances between consecutive points
bbox_width        = max(x) - min(x)
bbox_height       = max(y) - min(y)
bbox_diagonal     = sqrt(bbox_width² + bbox_height²)
aspect_ratio      = bbox_width / bbox_height
endpoint_distance = dist(first_point, last_point)
closedness        = endpoint_distance / path_length  (0 = closed, 1 = straight)
sinuosity         = path_length / endpoint_distance  (1 = straight, higher = winding)
linearity         = endpoint_distance / path_length  (inverse of sinuosity)
direction_changes = count of sign changes in angle between consecutive segments
curvature_sum     = sum of absolute angle changes between consecutive segments
curvature_mean    = curvature_sum / num_segments
sharpness         = sum of squared angle changes (Rubine f11)
```

### Classification Signatures

| Feature | Handwriting | Shape | Decorative |
|---------|-------------|-------|------------|
| Closedness | Medium (0.2-0.6) | Near 0 (closed) or near 1 (lines) | Varies |
| Linearity | Low (<0.5, winding) | High (>0.9 for lines) | Medium |
| Direction changes / length | High (complex curves) | Low (simple geometry) | Low-medium |
| Bounding box size | Small, consistent | Large relative to canvas | Small |
| Stroke clustering | Tight clusters on baseline | Sparse, spread out | Isolated |
| Sinuosity | High (>2.0) | Low (~1.0 for lines) | High for squiggles |
| Sharpness | High (many sharp turns) | Low (smooth/geometric) | Low-medium |

## Decision Tree (No ML Required)

```
classify(stroke, nearby_strokes):
    features = compute_features(stroke)

    // Step 1: Geometric shapes
    if features.closedness < 0.15 AND curvature_variance < threshold:
        if aspect_ratio ∈ [0.8, 1.2] AND corners == 0:
            return SHAPE(circle/ellipse)
        if corners == 4 AND closedness < 0.1:
            return SHAPE(rectangle)
        if corners == 3:
            return SHAPE(triangle)

    // Step 2: Straight lines and arrows
    if features.linearity > 0.92 AND curvature_mean < 5°:
        if has_arrowhead_at_endpoint(stroke):
            return SHAPE(arrow)
        return SHAPE(line)

    // Step 3: Decorative marks
    if features.path_length < min_text_threshold:
        return DECORATIVE(dot/mark)
    if linearity > 0.7 AND bbox_height < small AND near_text_baseline:
        return DECORATIVE(underline)
    if sinuosity > 3.0 AND bbox_height < small:
        return DECORATIVE(squiggle)

    // Step 4: Handwriting
    if clusters_with_similar_strokes(nearby)
       AND aligns_to_baseline(nearby)
       AND direction_changes / path_length > text_threshold:
        return TEXT

    // Step 5: Complex fallback
    if curvature_sum > high AND direction_changes > many:
        return TEXT

    return DECORATIVE  // default for unclassifiable
```

## How Existing Systems Do It

### Microsoft InkAnalyzer

- **Patent**: US20080292190A1 — patents.google.com/patent/US20080292190
- **Docs**: learn.microsoft.com/en-us/windows/win32/tablet/ink-analysis-overview
- Two-stage pipeline:
  1. **Stroke-level**: Neural network classifies each stroke as "writing" or "drawing" using normalized length, curvature, regression fit confidence, fragment analysis, and major axis orientation
  2. **Line-level**: HMM with Viterbi decoding reclassifies stroke clusters, catching misclassifications
- Produces hierarchy: Root → WritingRegion → Paragraph → Line → Word + Drawing nodes
- Recognizes shape primitives (circles, triangles, rectangles, trapezoids)

### Google ML Kit Digital Ink Recognition

- **Docs**: developers.google.com/ml-kit/vision/digital-ink-recognition
- Takes `Ink` object with `Stroke` → `StrokePoint(x, y, timestamp)` entries
- Returns recognized text + confidence score
- Supports 300+ languages, runs on-device (~20MB model, ~100ms)
- Classifies: text, emojis, basic shapes, gestures (9 gesture classes)
- Does NOT return per-stroke segmentation (which strokes formed which text)

### MyScript iink SDK

- **Docs**: myscript.com/ai/, developer.myscript.com
- Uses Graph Neural Networks (GNNs) for stroke classification
- Multiple GNN layers extract increasingly global features
- Can determine a leftmost stroke is part of a square while a neighboring stroke is part of "T"
- Recognizes text (70+ languages), math, diagrams, shapes, emojis, music notation

### Apple PencilKit

- Rich stroke data (position, timestamp, force, tilt, altitude)
- Does NOT expose a public classification API
- Uses spline-based recognition internally for Notes app

### W3C Handwriting Recognition API (Proposed)

- **Spec**: github.com/WICG/handwriting-recognition
- Web-standard API for on-line handwriting recognition
- Takes `HandwritingStroke` with `HandwritingPoint(x, y, t)`
- Provides segmentation mapping graphemes to composing strokes/points
- Behind a flag in Chromium

## Academic Papers

### Text/Non-Text Stroke Classification

1. **Zhou & Liu (2007)** — "Text/Non-text Ink Stroke Classification in Japanese Handwriting Based on Markov Random Fields"
   - ieeexplore.ieee.org/document/4378735
   - MRF + SVM with 114 features → **>96% accuracy** on IAM-OnDo database
   - Key: spatial relationships between strokes matter enormously

2. **Avola et al. (2017/2020)** — "Online Separation of Handwriting from Freehand Drawing"
   - springer.com/chapter/10.1007/978-3-319-68560-1_20 (2017)
   - springer.com/article/10.1007/s11042-019-7196-1 (2020)
   - SVM classifier → **97.3% accuracy** on text vs. drawing
   - Novel discriminative feature set
   - Compares SVM vs. ELM (Extreme Learning Machine)

3. **Indermuhle et al. (2013)** — "Contextual text/non-text stroke classification with CRFs"
   - sciencedirect.com/science/article/abs/pii/S0031320313001878
   - CRF-based approach using contextual features

### Graph Neural Network Approaches

4. **Ye et al. (2020)** — "Contextual Stroke Classification with Graph Attention Networks"
   - ieeexplore.ieee.org/document/8978003
   - GAT for node classification: strokes = nodes, edges = temporal/spatial relationships
   - Classifies: text, drawing, formula

5. **Ye et al. (2020)** — "Edge Graph Attention Networks for Stroke Classification"
   - springer.com/article/10.1007/s42979-020-00177-0
   - EGAT incorporates relational data from neighboring strokes

6. **Li et al. (2023)** — "DyGAT: Dynamic Stroke Classification"
   - sciencedirect.com/science/article/abs/pii/S0031320323002649
   - Dynamic graph attention network for real-time prediction
   - Multi-feature graph construction per stroke

7. **Ott et al. (2024)** — "Transformer-based Stroke Relation Encoding"
   - sciencedirect.com/science/article/abs/pii/S0031320323008282
   - Novel relation encoding for Transformers on stroke data

### Shape Recognition

8. **PaleoSketch (Paulson & Hammond, 2008)** — "Accurate Primitive Sketch Recognition and Beautification"
   - researchgate.net/publication/221607733
   - Recognizes 8 primitives (line, polyline, circle, ellipse, arc, curve, spiral, helix) at **98.56% accuracy**
   - Uses direction graph, speed graph, curvature graph, corner detection
   - Key features: NDDE (Normalized Distance between Direction Extremes), DCR (Direction Change Ratio)

9. **Paulson & Hammond (2008)** — "What!?! No Rubine Features?"
   - psi.engr.tamu.edu/wp-content/uploads/2018/01/paulson2008no.pdf
   - 31 geometric features that outperform Rubine's 13 on freely-sketched data
   - Adds: NDDE, DCR, perimeter-to-area ratio, endpoint-to-length ratio

### Comprehensive Feature Sets

10. **Blagojevic & Plimmer (2008-2010)** — 119-feature ink library ("Rubine on Steroids")
    - diglib.eg.org (2008), springer.com/chapter/10.1007/978-3-642-13022-9_36 (2010)
    - Features organized into: size, ratio, angle, distance, curvature, density categories
    - Used for diagram recognition

### Foundation Models

11. **InkFM (2025)** — "A Foundational Model for Full-Page Online Handwritten Note Understanding"
    - arxiv.org/abs/2503.23081
    - Fine-tuned PaliGemma for full-page ink analysis
    - Segments pages into text, diagrams, tables, drawings
    - State-of-the-art text line segmentation
    - Recognizes 28 scripts + math expressions

### Sketch Generation/Encoding

12. **Ha & Eck (2017)** — "A Neural Representation of Sketch Drawings" (sketch-rnn)
    - arxiv.org/abs/1704.03477
    - Stroke encoding: `(dx, dy, p1, p2, p3)` with 3-state pen
    - Bidirectional RNN encoder, HyperLSTM decoder
    - Trained on Quick, Draw! dataset (50M drawings, 345 categories)

## Open Source Libraries

### Gesture/Shape Recognizers

- **$1 Unistroke Recognizer** — depts.washington.edu/acelab/proj/dollar/index.html
  - Template-based geometric matcher, ~100 lines of code
  - Algorithm: resample → rotate → scale → translate → path-distance match
  - Wobbrock et al. (2007), UIST

- **$P Point-Cloud Recognizer** — depts.washington.edu/acelab/proj/dollar/pdollar.html
  - Ignores stroke number, order, and direction
  - Treats gestures as unordered point clouds

- **shape-detector** — github.com/MathieuLoutre/shape-detector (JavaScript, npm)
  - Based on $1 Recognizer

### Stroke Processing

- **perfect-freehand** — github.com/steveruizok/perfect-freehand (JavaScript)
  - Generates outline points for pressure-sensitive strokes
  - Used by Canva, draw.io, Excalidraw, tldraw

- **Google Ink Stroke Modeler** — github.com/google/ink-stroke-modeler (C++)
  - Smooths raw freehand input, predicts motion to minimize latency

### Datasets

- **CASIA-onDo** — nlpr.ia.ac.cn/databases/CASIA-onDo
  - 2,012 documents, 200 writers, 6 content types, 11 semantic labels
  - Each stroke: (x, y, pressure, pen-state, timestamp)

- **Quick, Draw!** — github.com/googlecreativelab/quickdraw-dataset
  - 50M drawings across 345 categories

- **IAM-OnDo** — Online handwritten document database

## Proposed Architecture for Nexor

```
Strokes from node
  → Compute geometric features (closedness, linearity, sinuosity, curvature, direction changes)
  → Heuristic classifier (no ML, no external deps)
      ├── SHAPE      →  RDP → JSON coordinates (~50 tokens)
      ├── TEXT        →  Google Ink Recognition → plain text (~5 tokens)
      └── DECORATIVE  →  label only ("underline near node X") (~2 tokens)
```

### Implementation Plan

1. **New module**: `src/server/hub/board_serializer/stroke_features.rs`
   - `compute_features(points: &[[f64; 2]]) -> StrokeFeatures`
   - Pure math on coordinates — closedness, linearity, sinuosity, curvature stats, direction changes, bbox

2. **New module**: `src/server/hub/board_serializer/stroke_classify.rs`
   - `classify_stroke(features: &StrokeFeatures, nearby: &[StrokeFeatures]) -> StrokeClass`
   - Decision tree from heuristics above
   - `enum StrokeClass { Shape, Text, Decorative(DecorativeKind) }`

3. **Integration in snapshot.rs**:
   - After collecting `node_strokes`, compute features and classify
   - Route each group to the appropriate encoder
   - Shape strokes → `encode::encode_strokes()` (already built)
   - Text strokes → future Google Ink integration
   - Decorative strokes → label string

4. **Google Ink Recognition** (future, requires API key):
   - Send TEXT-classified strokes to Google Digital Ink Recognition
   - Store recognized text in `CanvasNode.stroke_text: Option<String>`

### Key Insight from Literature

> "Contextual features matter enormously. Isolated stroke classification tops out around 90-95% accuracy. Adding spatial and temporal context (neighboring strokes, baseline alignment, temporal grouping) pushes accuracy above 97%." — Zhou & Liu (2007), Avola et al. (2020)

We don't have timestamps from Excalidraw, but we DO have spatial context (nearby strokes, node bounding boxes). That's enough for the heuristic approach.

# Notch-Based Text Scaling Algorithm for Dynamic Nodes

## Context

The current text scaling in `CanvasFormNode` uses a continuous `Math.pow(width / 560, 0.5)` formula applied via CSS `zoom`. This produces two problems:
1. **Width-only** — a node stretched wide but kept short (terminal-like) gets enormous text
2. **Continuous** — the zoom changes every pixel during drag, causing jelly-like re-renders

The fix: replace the continuous width-only formula with a **discrete notch system** that considers **both width and height independently**, picking the more conservative of the two.

## Algorithm

Six discrete notches. The final notch = `min(widthNotch, heightNotch)`.

| Notch | Zoom  | When It Applies |
|-------|-------|-----------------|
| XS    | 0.85  | Cramped or one dimension very small |
| S     | 0.95  | Below default |
| **M** | **1.0** | **Default (560x500) — no change** |
| L     | 1.12  | Moderately larger |
| XL    | 1.25  | Large |
| XXL   | 1.4   | Near max size in both dimensions |

**Width breakpoints:**
| Range | Notch |
|-------|-------|
| <= 420 | XS |
| 421-559 | S |
| 560-780 | M |
| 781-1050 | L |
| 1051-1400 | XL |
| 1401+ | XXL |

**Height breakpoints:**
| Range | Notch |
|-------|-------|
| <= 350 | XS |
| 351-499 | S |
| 500-700 | M |
| 701-1000 | L |
| 1001-1300 | XL |
| 1301+ | XXL |

**Key scenario fix:** A 1800x300 node (wide+short) now gets `min(XXL, XS) = XS = 0.85` instead of the current `1.79`.

**Example scenarios:**

| Width | Height | W-Notch | H-Notch | Final | Zoom | Behavior |
|-------|--------|---------|---------|-------|------|----------|
| 360 | 300 | XS | XS | XS | 0.85 | Min size: text shrinks slightly |
| 560 | 500 | M | M | M | 1.0 | Default: unchanged from design |
| 1800 | 300 | XXL | XS | **XS** | **0.85** | Wide+short: height caps it |
| 360 | 1600 | XS | XXL | **XS** | **0.85** | Narrow+tall: width caps it |
| 1200 | 1200 | XL | XL | XL | 1.25 | Large both ways: comfortable |
| 1800 | 1600 | XXL | XXL | XXL | 1.4 | Max size (was 1.79, now 1.4) |
| 800 | 400 | M | S | **S** | **0.95** | Medium wide, short: height wins |

## Files to Modify

### 1. `frontend/src/components/canvas/CanvasFormNode/constants.ts`
- Add `ScaleNotch` type, `NOTCH_ORDER` array, `SCALE_NOTCH_ZOOM` map
- Add `WIDTH_BREAKPOINTS` and `HEIGHT_BREAKPOINTS` arrays
- All tuning lives here — change a number, change the behavior

### 2. NEW: `frontend/src/components/canvas/CanvasFormNode/scaleNotch.ts`
- Pure utility: `resolveScaleNotch(width, height)` and `resolveScaleFactor(width, height)`
- Linear scan of breakpoints, pick min of width/height notch indices
- No React dependencies — trivially testable

### 3. `frontend/src/components/canvas/CanvasFormNode/CanvasFormNode.tsx`
- Import `resolveScaleFactor` from `./scaleNotch`
- Update `ResizeObserver` to read both `width` and `height` from `contentRect`
- Replace `Math.pow` formula with `resolveScaleFactor(width, height)`
- Add state-equality check to avoid re-renders when notch hasn't changed: `setScaleFactor((prev) => prev === next ? prev : next)`
- The `zoom: scaleFactor` wrapper stays as-is

### 4. NEW: `frontend/src/components/canvas/CanvasFormNode/scaleNotch.test.ts`
- Pure function tests covering: min size, default size, wide+short, narrow+tall, max size, height-constrained scenarios

### 5. `frontend/src/components/canvas/CanvasFormNode/index.ts`
- Export `resolveScaleNotch` and `resolveScaleFactor` from barrel

## Why Keep CSS `zoom`

CSS custom properties would require touching 10+ child components to replace hardcoded font sizes. CSS `zoom` scales everything uniformly (fonts, padding, icons, borders) from a single point. The only change needed is how we compute the zoom value.

## Performance Benefit

Discrete notches mean `setScaleFactor` only triggers a re-render when the notch actually changes (typically 0-2 times during a resize drag). The current continuous approach re-renders on every frame. The state-equality check (`prev === next ? prev : next`) makes this explicit.

## Tuning Guide

- **Make text bigger at a given size:** Lower the preceding breakpoint's max value
- **Add a notch between M and L:** Add entry to `NOTCH_ORDER`, `SCALE_NOTCH_ZOOM`, and both breakpoint arrays
- **Change zoom at a notch:** Edit `SCALE_NOTCH_ZOOM` values
- **Disable downscaling:** Set XS and S zoom values to `1.0`
- All tuning is confined to `constants.ts`

## Verification

1. `npx tsc --noEmit` — type check
2. `npx eslint .` — lint (zero warnings)
3. `npx vitest run src/components/canvas/CanvasFormNode/scaleNotch.test.ts` — unit tests
4. Manual testing in browser:
   - Default 560x500 node: text unchanged (zoom 1.0)
   - Drag wide + short (e.g. 1400x350): text stays small (XS/S, not huge)
   - Drag both dimensions large (1200x1200): text scales up comfortably (XL = 1.25)
   - Max size 1800x1600: text at XXL (1.4), not the old 1.79

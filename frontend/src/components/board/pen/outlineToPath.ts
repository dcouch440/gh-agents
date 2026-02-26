// ============================================================================
// outlineToPath — Convert Outline Points to Canvas2D Path Commands
// ============================================================================
//
// Takes the polygon outline from `getStrokeOutline` and renders it as a
// smooth filled shape using quadratic bezier curves between midpoints.

import type { Point } from '@/utils/geometry'

/**
 * Render filled outline points onto a Canvas2D context using quadratic
 * bezier curves for smooth edges.
 *
 * This draws directly to `ctx` rather than building a path string — avoids
 * the overhead of Path2D parsing and produces identical visual output.
 */
const fillOutlinePath = (
  ctx: CanvasRenderingContext2D,
  outline: readonly Point[],
): void => {
  if (outline.length < 3) return

  ctx.beginPath()
  ctx.moveTo(outline[0]!.x, outline[0]!.y)

  for (let i = 1; i < outline.length - 1; i++) {
    const curr = outline[i]!
    const next = outline[i + 1]!
    const midX = (curr.x + next.x) / 2
    const midY = (curr.y + next.y) / 2
    ctx.quadraticCurveTo(curr.x, curr.y, midX, midY)
  }

  // Close to the last point
  const last = outline[outline.length - 1]!
  ctx.lineTo(last.x, last.y)

  ctx.closePath()
  ctx.fill()
}

export { fillOutlinePath }

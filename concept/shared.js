// ============================================================================
// Shared helpers for arrow concept pages
// ============================================================================

const BG = '#1e1e2e'
const DARK_BG = '#181825'
const SURFACE = '#313244'
const STROKE = '#cdd6f4'
const MUTED = '#6c7086'
const ACCENT = '#89b4fa'

// Simple seeded PRNG for consistent randomness
function mulberry32(a) {
  return function() {
    a |= 0; a = a + 0x6D2B79F5 | 0
    var t = Math.imul(a ^ a >>> 15, 1 | a)
    t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t
    return ((t ^ t >>> 14) >>> 0) / 4294967296
  }
}

function drawRoughBox(ctx, x, y, w, h, seed) {
  const rng = mulberry32(seed)
  const r = 14
  const roughness = 1.2

  ctx.fillStyle = SURFACE
  ctx.beginPath()
  ctx.roundRect(x, y, w, h, r)
  ctx.fill()

  ctx.strokeStyle = STROKE
  ctx.lineWidth = 2
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'

  for (let pass = 0; pass < 2; pass++) {
    ctx.beginPath()
    const points = getRoundedRectPoints(x, y, w, h, r)
    for (let i = 0; i < points.length; i++) {
      const p = points[i]
      const jx = (rng() - 0.5) * roughness * (pass === 0 ? 1 : -0.5)
      const jy = (rng() - 0.5) * roughness * (pass === 0 ? 1 : -0.5)
      if (i === 0) ctx.moveTo(p.x + jx, p.y + jy)
      else ctx.lineTo(p.x + jx, p.y + jy)
    }
    ctx.closePath()
    ctx.globalAlpha = pass === 0 ? 1 : 0.3
    ctx.stroke()
  }
  ctx.globalAlpha = 1
}

function getRoundedRectPoints(x, y, w, h, r) {
  const pts = []
  const steps = 6
  for (let i = 0; i <= 10; i++) pts.push({ x: x + r + (w - 2*r) * i/10, y: y })
  for (let i = 0; i <= steps; i++) {
    const a = -Math.PI/2 + (Math.PI/2) * i/steps
    pts.push({ x: x + w - r + r*Math.cos(a), y: y + r + r*Math.sin(a) })
  }
  for (let i = 0; i <= 10; i++) pts.push({ x: x + w, y: y + r + (h - 2*r) * i/10 })
  for (let i = 0; i <= steps; i++) {
    const a = 0 + (Math.PI/2) * i/steps
    pts.push({ x: x + w - r + r*Math.cos(a), y: y + h - r + r*Math.sin(a) })
  }
  for (let i = 0; i <= 10; i++) pts.push({ x: x + w - r - (w - 2*r) * i/10, y: y + h })
  for (let i = 0; i <= steps; i++) {
    const a = Math.PI/2 + (Math.PI/2) * i/steps
    pts.push({ x: x + r + r*Math.cos(a), y: y + h - r + r*Math.sin(a) })
  }
  for (let i = 0; i <= 10; i++) pts.push({ x: x, y: y + h - r - (h - 2*r) * i/10 })
  for (let i = 0; i <= steps; i++) {
    const a = Math.PI + (Math.PI/2) * i/steps
    pts.push({ x: x + r + r*Math.cos(a), y: y + r + r*Math.sin(a) })
  }
  return pts
}

function drawText(ctx, text, x, y, w) {
  ctx.fillStyle = STROKE
  ctx.font = '16px Virgil, Segoe Print, Bradley Hand, system-ui, sans-serif'
  ctx.textBaseline = 'middle'
  ctx.textAlign = 'center'
  ctx.fillText(text, x + w/2, y)
}

function bezierPoint(t, p0, p1, p2, p3) {
  const u = 1 - t
  return {
    x: u*u*u*p0.x + 3*u*u*t*p1.x + 3*u*t*t*p2.x + t*t*t*p3.x,
    y: u*u*u*p0.y + 3*u*u*t*p1.y + 3*u*t*t*p2.y + t*t*t*p3.y,
  }
}

function bezierTangent(t, p0, p1, p2, p3) {
  const u = 1 - t
  return {
    x: 3*u*u*(p1.x-p0.x) + 6*u*t*(p2.x-p1.x) + 3*t*t*(p3.x-p2.x),
    y: 3*u*u*(p1.y-p0.y) + 6*u*t*(p2.y-p1.y) + 3*t*t*(p3.y-p2.y),
  }
}

function controlPoint(pt, side, dist) {
  switch (side) {
    case 'right':  return { x: pt.x + dist, y: pt.y }
    case 'left':   return { x: pt.x - dist, y: pt.y }
    case 'top':    return { x: pt.x, y: pt.y - dist }
    case 'bottom': return { x: pt.x, y: pt.y + dist }
  }
}

// Standard 3-box scene used by all concepts
function getScene(gap) {
  const boxes = [
    { x: 40,  y: 60,  w: 180, h: 56, text: 'Research' },
    { x: 40,  y: 240, w: 200, h: 56, text: 'Decompose' },
    { x: 360, y: 150, w: 180, h: 56, text: 'Evaluate' },
  ]

  const arrows = [
    {
      start: { x: boxes[0].x + boxes[0].w + gap, y: boxes[0].y + boxes[0].h / 2 },
      end:   { x: boxes[2].x - gap, y: boxes[2].y + boxes[2].h * 0.35 },
      startSide: 'right', endSide: 'left',
    },
    {
      start: { x: boxes[1].x + boxes[1].w + gap, y: boxes[1].y + boxes[1].h / 2 },
      end:   { x: boxes[2].x - gap, y: boxes[2].y + boxes[2].h * 0.65 },
      startSide: 'right', endSide: 'left',
    },
    {
      start: { x: boxes[0].x + boxes[0].w * 0.5, y: boxes[0].y + boxes[0].h + gap },
      end:   { x: boxes[1].x + boxes[1].w * 0.5, y: boxes[1].y - gap },
      startSide: 'bottom', endSide: 'top',
    },
  ]

  for (const a of arrows) {
    const dist = Math.sqrt((a.end.x - a.start.x)**2 + (a.end.y - a.start.y)**2)
    const cpDist = Math.max(40, dist * 0.4)
    a.cp1 = controlPoint(a.start, a.startSide, cpDist)
    a.cp2 = controlPoint(a.end, a.endSide, cpDist)
  }

  return { boxes, arrows }
}

// Bigger scene — 5 boxes, more complex topology
function getBigScene(gap) {
  const boxes = [
    { x: 40,  y: 40,  w: 160, h: 50, text: 'Ingest' },
    { x: 40,  y: 200, w: 160, h: 50, text: 'Parse' },
    { x: 280, y: 40,  w: 180, h: 50, text: 'Classify' },
    { x: 280, y: 200, w: 180, h: 50, text: 'Validate' },
    { x: 530, y: 120, w: 160, h: 50, text: 'Output' },
  ]

  const arrows = [
    // Ingest -> Classify
    {
      start: { x: boxes[0].x + boxes[0].w + gap, y: boxes[0].y + boxes[0].h / 2 },
      end:   { x: boxes[2].x - gap, y: boxes[2].y + boxes[2].h / 2 },
      startSide: 'right', endSide: 'left',
    },
    // Ingest -> Parse (down)
    {
      start: { x: boxes[0].x + boxes[0].w * 0.5, y: boxes[0].y + boxes[0].h + gap },
      end:   { x: boxes[1].x + boxes[1].w * 0.5, y: boxes[1].y - gap },
      startSide: 'bottom', endSide: 'top',
    },
    // Parse -> Validate
    {
      start: { x: boxes[1].x + boxes[1].w + gap, y: boxes[1].y + boxes[1].h / 2 },
      end:   { x: boxes[3].x - gap, y: boxes[3].y + boxes[3].h / 2 },
      startSide: 'right', endSide: 'left',
    },
    // Classify -> Output
    {
      start: { x: boxes[2].x + boxes[2].w + gap, y: boxes[2].y + boxes[2].h / 2 },
      end:   { x: boxes[4].x - gap, y: boxes[4].y + boxes[4].h * 0.35 },
      startSide: 'right', endSide: 'left',
    },
    // Validate -> Output
    {
      start: { x: boxes[3].x + boxes[3].w + gap, y: boxes[3].y + boxes[3].h / 2 },
      end:   { x: boxes[4].x - gap, y: boxes[4].y + boxes[4].h * 0.65 },
      startSide: 'right', endSide: 'left',
    },
  ]

  for (const a of arrows) {
    const dist = Math.sqrt((a.end.x - a.start.x)**2 + (a.end.y - a.start.y)**2)
    const cpDist = Math.max(40, dist * 0.4)
    a.cp1 = controlPoint(a.start, a.startSide, cpDist)
    a.cp2 = controlPoint(a.end, a.endSide, cpDist)
  }

  return { boxes, arrows }
}

// Init a concept canvas — returns { canvas, ctx }
function initCanvas(id, width, height) {
  const canvas = document.getElementById(id)
  const ctx = canvas.getContext('2d')
  canvas.width = width * 2
  canvas.height = height * 2
  ctx.scale(2, 2)
  ctx.clearRect(0, 0, width, height)
  return { canvas, ctx }
}

function drawBoxes(ctx, boxes, seed) {
  for (let i = 0; i < boxes.length; i++) {
    const b = boxes[i]
    drawRoughBox(ctx, b.x, b.y, b.w, b.h, seed + i * 13)
    drawText(ctx, b.text, b.x, b.y + b.h/2, b.w)
  }
}

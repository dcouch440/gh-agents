// ---------------------------------------------------------------------------
// Panel markdown parser
// ---------------------------------------------------------------------------
// Converts markdown with headings and checkboxes into a structured
// PanelSection tree for rendering as nested cards.
//
// Only `- [ ]` / `- [x]` checkboxes are interactive. Regular markdown
// (bullets, paragraphs, tables, etc.) flows into bodyMarkdown for
// TerminalBlock rendering.

type PanelInteractiveItem = {
  type: 'checkbox'
  label: string
  checked: boolean
  id: string
}

type PanelSection = {
  id: string
  depth: number // 0 = H1, 1 = H2, 2 = H3
  title: string
  bodyMarkdown: string
  interactiveItems: PanelInteractiveItem[]
  children: PanelSection[]
}

let idCounter = 0
const nextId = (): string => `panel-${++idCounter}`

const resetIdCounter = (): void => {
  idCounter = 0
}

const HEADING_RE = /^(#{1,3})\s+(.*)$/
const CHECKBOX_RE = /^-\s+\[([ xX])\]\s+(.*)$/

const parsePanel = (markdown: string): PanelSection[] => {
  resetIdCounter()

  if (!markdown.trim()) return []

  const lines = markdown.split('\n')

  // First pass: identify heading boundaries and create flat sections
  type FlatSection = {
    id: string
    depth: number
    title: string
    bodyLines: string[]
    interactiveItems: PanelInteractiveItem[]
  }

  const flatSections: FlatSection[] = []
  let current: FlatSection | null = null

  for (const line of lines) {
    const headingMatch = HEADING_RE.exec(line)
    if (headingMatch) {
      // Start a new section
      current = {
        id: nextId(),
        depth: headingMatch[1].length - 1, // # = 0, ## = 1, ### = 2
        title: headingMatch[2],
        bodyLines: [],
        interactiveItems: [],
      }
      flatSections.push(current)
      continue
    }

    if (!current) {
      // Content before any heading — create an implicit root section
      current = {
        id: nextId(),
        depth: 0,
        title: '',
        bodyLines: [],
        interactiveItems: [],
      }
      flatSections.push(current)
    }

    // Check for interactive elements
    const checkboxMatch = CHECKBOX_RE.exec(line)
    if (checkboxMatch) {
      current.interactiveItems.push({
        type: 'checkbox',
        label: checkboxMatch[2],
        checked: checkboxMatch[1] !== ' ',
        id: nextId(),
      })
      continue
    }

    // Regular body line (including plain `- item` bullets)
    current.bodyLines.push(line)
  }

  // Convert flat sections to final format
  const toSection = (flat: FlatSection): PanelSection => ({
    id: flat.id,
    depth: flat.depth,
    title: flat.title,
    bodyMarkdown: flat.bodyLines.join('\n').trim(),
    interactiveItems: flat.interactiveItems,
    children: [],
  })

  // Build nested tree
  const roots: PanelSection[] = []
  const stack: PanelSection[] = []

  for (const flat of flatSections) {
    const section = toSection(flat)

    // Pop stack until we find a parent with lower depth
    while (stack.length > 0 && stack[stack.length - 1].depth >= section.depth) {
      stack.pop()
    }

    if (stack.length > 0) {
      stack[stack.length - 1].children.push(section)
    } else {
      roots.push(section)
    }

    stack.push(section)
  }

  return roots
}

export { parsePanel, resetIdCounter }
export type { PanelSection, PanelInteractiveItem }

// ============================================================================
// AsciiTree — Hierarchical Tree Builder + Box-Drawing Renderer
// ============================================================================

/**
 * Builds hierarchical trees from flat data and renders them as ASCII text
 * using box-drawing characters (├── └── │).
 *
 * Designed for monospace display — output renders cleanly in TerminalBlock,
 * code fences, or any fixed-width context.
 *
 * Generic over input type: supply extractors for id, label, parentId, and
 * an optional detail line. The class handles tree construction, orphan
 * recovery, cycle avoidance, and sibling sorting.
 *
 * @example
 * ```ts
 * const tree = AsciiTree.from(agents, {
 *   id: (a) => a.id,
 *   label: (a) => a.name,
 *   parentId: (a) => a.depends_on[0] ?? null,
 *   detail: (a) => a.capabilities.join(', ') || null,
 * })
 *
 * tree.render()
 * // ├── Lead Researcher
 * // │   search, analyze
 * // │   ├── Web Searcher
 * // │   │   web_search
 * // │   └── Fact Checker
 * // │       verification
 * // └── Reviewer
 * //     review, summarize
 * ```
 */

// ── Types ────────────────────────────────────────────────────────────────

type TreeNode = {
  readonly id: string
  readonly label: string
  readonly detail: string | null
  readonly children: readonly TreeNode[]
}

/** Mutable variant used during tree construction. */
type MutableTreeNode = {
  readonly id: string
  readonly label: string
  readonly detail: string | null
  readonly children: MutableTreeNode[]
}

type AsciiTreeConfig<T> = {
  /** Unique identifier for each item. */
  id: (item: T) => string
  /** Display text for the tree line. */
  label: (item: T) => string
  /** Parent's id, or null for root nodes. */
  parentId: (item: T) => string | null
  /** Optional secondary line rendered below the label, indented to match. */
  detail?: (item: T) => string | null
  /** Sort comparator for siblings. Default: insertion order. */
  sortBy?: (a: T, b: T) => number
}

// ── Box-Drawing Constants ────────────────────────────────────────────────

const BRANCH = '├── '
const CORNER = '└── '
const PIPE = '│   '
const SPACE = '    '
const SINGLE = '── '
const SINGLE_INDENT = '   '

// ── Render Helpers ───────────────────────────────────────────────────────

const renderNodes = (nodes: readonly TreeNode[], prefix: string): string[] => {
  const lines: string[] = []
  const n = nodes.length

  for (let i = 0; i < n; i++) {
    const node = nodes[i]!
    const isLast = i === n - 1
    const connector = isLast ? CORNER : BRANCH
    const childPrefix = prefix + (isLast ? SPACE : PIPE)

    lines.push(prefix + connector + node.label)

    if (node.detail) {
      lines.push(childPrefix + node.detail)
    }

    if (node.children.length > 0) {
      const childLines = renderNodes(node.children, childPrefix)
      const cn = childLines.length
      for (let j = 0; j < cn; j++) {
        lines.push(childLines[j]!)
      }
    }
  }

  return lines
}

// ── AsciiTree ────────────────────────────────────────────────────────────

class AsciiTree {
  private constructor(private readonly roots: readonly TreeNode[]) {}

  // ── Static Factories ─────────────────────────────────────────────────

  /**
   * Build a tree from a flat array of items.
   *
   * Construction rules:
   * - Items with `parentId === null` are roots.
   * - Items whose `parentId` references another item become children.
   * - Items whose `parentId` references a non-existent item are promoted
   *   to roots (defensive — never silently drop nodes).
   * - Cycles are avoided: a node is only attached once.
   *
   * O(n) construction — single pass to index, single pass to link.
   */
  static from<T>(items: readonly T[], config: AsciiTreeConfig<T>): AsciiTree {
    const n = items.length
    if (n === 0) return new AsciiTree([])

    const detailFn = config.detail ?? (() => null)

    // Pass 1: Create all MutableTreeNodes and index by id
    const nodeMap = new Map<string, MutableTreeNode>()
    const itemById = new Map<string, T>()

    for (let i = 0; i < n; i++) {
      const item = items[i]!
      const id = config.id(item)
      const rawDetail = detailFn(item)
      const node: MutableTreeNode = {
        id,
        label: config.label(item),
        detail: rawDetail ?? null,
        children: [],
      }
      nodeMap.set(id, node)
      itemById.set(id, item)
    }

    // Pass 2: Link children to parents, collect roots
    const roots: MutableTreeNode[] = []
    const attached = new Set<string>()

    for (let i = 0; i < n; i++) {
      const item = items[i]!
      const id = config.id(item)
      const node = nodeMap.get(id)!
      const parentId = config.parentId(item)

      if (parentId !== null && parentId !== id) {
        const parent = nodeMap.get(parentId)
        if (parent && !attached.has(id)) {
          parent.children.push(node)
          attached.add(id)
          continue
        }
      }

      // Root: no parent, self-referencing parent, or orphaned parent
      if (!attached.has(id)) {
        roots.push(node)
        attached.add(id)
      }
    }

    // Pass 3: Sort siblings if comparator provided
    if (config.sortBy) {
      const comparator = config.sortBy
      const sortChildren = (nodes: MutableTreeNode[]) => {
        nodes.sort((a, b) => {
          const itemA = itemById.get(a.id)
          const itemB = itemById.get(b.id)
          if (!itemA || !itemB) return 0
          return comparator(itemA, itemB)
        })
        const cn = nodes.length
        for (let i = 0; i < cn; i++) {
          sortChildren(nodes[i]!.children)
        }
      }
      sortChildren(roots)
    }

    return new AsciiTree(roots)
  }

  /**
   * Create a mutable builder for manual node-by-node construction.
   *
   * @example
   * ```ts
   * const tree = AsciiTree.builder()
   *   .addRoot('a', 'Root Node', 'some detail')
   *   .addChild('a', 'b', 'Child One')
   *   .addChild('a', 'c', 'Child Two')
   *   .build()
   * ```
   */
  static builder(): AsciiTreeBuilder {
    return new AsciiTreeBuilder()
  }

  // ── Rendering ────────────────────────────────────────────────────────

  /**
   * Render the tree as plain text with box-drawing characters.
   *
   * Single root with no siblings omits the leading connector:
   * ```
   * ── Root
   *    ├── Child A
   *    └── Child B
   * ```
   *
   * Multiple roots use standard connectors:
   * ```
   * ├── Root A
   * └── Root B
   * ```
   */
  render(): string {
    const n = this.roots.length
    if (n === 0) return ''

    if (n === 1) {
      const root = this.roots[0]!
      const lines: string[] = [SINGLE + root.label]

      if (root.detail) {
        lines.push(SINGLE_INDENT + root.detail)
      }

      if (root.children.length > 0) {
        const childLines = renderNodes(root.children, SINGLE_INDENT)
        const cn = childLines.length
        for (let i = 0; i < cn; i++) {
          lines.push(childLines[i]!)
        }
      }

      return lines.join('\n')
    }

    return renderNodes(this.roots, '').join('\n')
  }

  /**
   * Render wrapped in a markdown fenced code block.
   * Preserves whitespace and box-drawing characters in TerminalBlock.
   */
  renderMarkdown(): string {
    const rendered = this.render()
    if (!rendered) return ''
    return '```\n' + rendered + '\n```'
  }

  // ── Accessors ────────────────────────────────────────────────────────

  /** True if the tree contains no nodes. */
  get isEmpty(): boolean {
    return this.roots.length === 0
  }

  /** Total number of nodes across all depths. */
  get size(): number {
    let count = 0
    const walk = (nodes: readonly TreeNode[]) => {
      const n = nodes.length
      count += n
      for (let i = 0; i < n; i++) {
        walk(nodes[i]!.children)
      }
    }
    walk(this.roots)
    return count
  }
}

// ── AsciiTreeBuilder ─────────────────────────────────────────────────────

class AsciiTreeBuilder {
  private readonly nodeMap = new Map<string, MutableTreeNode>()
  private readonly roots: MutableTreeNode[] = []

  /** Add a root-level node. */
  addRoot(id: string, label: string, detail?: string | null): this {
    const node: MutableTreeNode = { id, label, detail: detail ?? null, children: [] }
    this.nodeMap.set(id, node)
    this.roots.push(node)
    return this
  }

  /** Add a child node under an existing parent. */
  addChild(parentId: string, id: string, label: string, detail?: string | null): this {
    const parent = this.nodeMap.get(parentId)
    if (!parent) {
      throw new Error(`AsciiTreeBuilder: parent "${parentId}" not found. Add parents before children.`)
    }
    const node: MutableTreeNode = { id, label, detail: detail ?? null, children: [] }
    this.nodeMap.set(id, node)
    parent.children.push(node)
    return this
  }

  /** Produce the immutable AsciiTree. */
  build(): AsciiTree {
    return AsciiTree.from(
      this.roots.flatMap(function collect(node: TreeNode): { id: string; label: string; detail: string | null; parentId: string | null }[] {
        const self = { id: node.id, label: node.label, detail: node.detail, parentId: null as string | null }
        const children = node.children.flatMap((child) => {
          const items = collect(child)
          items[0] = { ...items[0]!, parentId: node.id }
          return items
        })
        return [self, ...children]
      }),
      {
        id: (item) => item.id,
        label: (item) => item.label,
        parentId: (item) => item.parentId,
        detail: (item) => item.detail,
      },
    )
  }
}

// ── Exports ──────────────────────────────────────────────────────────────

export { AsciiTree, AsciiTreeBuilder }
export type { AsciiTreeConfig, TreeNode }

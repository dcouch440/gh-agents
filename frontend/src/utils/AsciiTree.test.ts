import { describe, it, expect } from 'vitest'
import { AsciiTree } from './AsciiTree'

// ── Helpers ──────────────────────────────────────────────────────────────

type Item = {
  id: string
  name: string
  parentId: string | null
  detail?: string
  order?: number
}

const config = {
  id: (i: Item) => i.id,
  label: (i: Item) => i.name,
  parentId: (i: Item) => i.parentId,
  detail: (i: Item) => i.detail ?? null,
}

const configWithSort = {
  ...config,
  sortBy: (a: Item, b: Item) => (a.order ?? 0) - (b.order ?? 0),
}

// ── from() construction ──────────────────────────────────────────────────

describe('AsciiTree.from', () => {
  it('returns empty tree for empty array', () => {
    const tree = AsciiTree.from([], config)
    expect(tree.isEmpty).toBe(true)
    expect(tree.size).toBe(0)
    expect(tree.render()).toBe('')
  })

  it('handles a single root node', () => {
    const tree = AsciiTree.from(
      [{ id: 'a', name: 'Root', parentId: null }],
      config,
    )
    expect(tree.isEmpty).toBe(false)
    expect(tree.size).toBe(1)
    expect(tree.render()).toBe('── Root')
  })

  it('handles a single root with detail', () => {
    const tree = AsciiTree.from(
      [{ id: 'a', name: 'Root', parentId: null, detail: 'some info' }],
      config,
    )
    expect(tree.render()).toBe(
      '── Root\n' +
      '   some info',
    )
  })

  it('renders flat siblings with connectors', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'Alpha', parentId: null },
        { id: 'b', name: 'Beta', parentId: null },
        { id: 'c', name: 'Gamma', parentId: null },
      ],
      config,
    )
    expect(tree.size).toBe(3)
    expect(tree.render()).toBe(
      '├── Alpha\n' +
      '├── Beta\n' +
      '└── Gamma',
    )
  })

  it('renders flat siblings with details', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'Alpha', parentId: null, detail: 'first' },
        { id: 'b', name: 'Beta', parentId: null, detail: 'second' },
      ],
      config,
    )
    expect(tree.render()).toBe(
      '├── Alpha\n' +
      '│   first\n' +
      '└── Beta\n' +
      '    second',
    )
  })

  it('renders nested children', () => {
    const tree = AsciiTree.from(
      [
        { id: 'root', name: 'Root', parentId: null },
        { id: 'c1', name: 'Child 1', parentId: 'root' },
        { id: 'c2', name: 'Child 2', parentId: 'root' },
      ],
      config,
    )
    expect(tree.size).toBe(3)
    expect(tree.render()).toBe(
      '── Root\n' +
      '   ├── Child 1\n' +
      '   └── Child 2',
    )
  })

  it('renders deep nesting (3 levels)', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'Level 0', parentId: null },
        { id: 'b', name: 'Level 1', parentId: 'a' },
        { id: 'c', name: 'Level 2', parentId: 'b' },
        { id: 'd', name: 'Level 3', parentId: 'c' },
      ],
      config,
    )
    expect(tree.size).toBe(4)
    expect(tree.render()).toBe(
      '── Level 0\n' +
      '   └── Level 1\n' +
      '       └── Level 2\n' +
      '           └── Level 3',
    )
  })

  it('renders complex mixed hierarchy', () => {
    const tree = AsciiTree.from(
      [
        { id: 'lead', name: 'Lead Researcher', parentId: null, detail: 'search, analyze' },
        { id: 'web', name: 'Web Searcher', parentId: 'lead', detail: 'web_search' },
        { id: 'paper', name: 'Paper Analyst', parentId: 'lead', detail: 'document_analysis' },
        { id: 'fact', name: 'Fact Checker', parentId: 'lead', detail: 'verification' },
        { id: 'rev', name: 'Reviewer', parentId: null, detail: 'review, summarize' },
      ],
      config,
    )
    expect(tree.size).toBe(5)
    expect(tree.render()).toBe(
      '├── Lead Researcher\n' +
      '│   search, analyze\n' +
      '│   ├── Web Searcher\n' +
      '│   │   web_search\n' +
      '│   ├── Paper Analyst\n' +
      '│   │   document_analysis\n' +
      '│   └── Fact Checker\n' +
      '│       verification\n' +
      '└── Reviewer\n' +
      '    review, summarize',
    )
  })

  it('promotes orphaned parentId references to roots', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'Alpha', parentId: 'nonexistent' },
        { id: 'b', name: 'Beta', parentId: null },
      ],
      config,
    )
    // Alpha's parent doesn't exist → promoted to root
    expect(tree.size).toBe(2)
    expect(tree.render()).toBe(
      '├── Alpha\n' +
      '└── Beta',
    )
  })

  it('handles self-referencing parentId', () => {
    const tree = AsciiTree.from(
      [{ id: 'a', name: 'Self', parentId: 'a' }],
      config,
    )
    // Self-referencing → treated as root
    expect(tree.size).toBe(1)
    expect(tree.render()).toBe('── Self')
  })

  it('applies sortBy to siblings', () => {
    const tree = AsciiTree.from(
      [
        { id: 'c', name: 'Gamma', parentId: null, order: 3 },
        { id: 'a', name: 'Alpha', parentId: null, order: 1 },
        { id: 'b', name: 'Beta', parentId: null, order: 2 },
      ],
      configWithSort,
    )
    expect(tree.render()).toBe(
      '├── Alpha\n' +
      '├── Beta\n' +
      '└── Gamma',
    )
  })

  it('applies sortBy to nested children', () => {
    const tree = AsciiTree.from(
      [
        { id: 'root', name: 'Root', parentId: null, order: 0 },
        { id: 'c', name: 'Third', parentId: 'root', order: 3 },
        { id: 'a', name: 'First', parentId: 'root', order: 1 },
        { id: 'b', name: 'Second', parentId: 'root', order: 2 },
      ],
      configWithSort,
    )
    expect(tree.render()).toBe(
      '── Root\n' +
      '   ├── First\n' +
      '   ├── Second\n' +
      '   └── Third',
    )
  })

  it('omits detail line for null/empty detail', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'No Detail', parentId: null },
        { id: 'b', name: 'Has Detail', parentId: null, detail: 'info' },
      ],
      config,
    )
    expect(tree.render()).toBe(
      '├── No Detail\n' +
      '└── Has Detail\n' +
      '    info',
    )
  })

  it('omits detail for empty string', () => {
    const tree = AsciiTree.from(
      [{ id: 'a', name: 'Root', parentId: null, detail: '' }],
      config,
    )
    // Empty string detail → treated as null (falsy)
    expect(tree.render()).toBe('── Root')
  })
})

// ── renderMarkdown() ─────────────────────────────────────────────────────

describe('AsciiTree.renderMarkdown', () => {
  it('returns empty string for empty tree', () => {
    const tree = AsciiTree.from([], config)
    expect(tree.renderMarkdown()).toBe('')
  })

  it('wraps render output in code fence', () => {
    const tree = AsciiTree.from(
      [{ id: 'a', name: 'Root', parentId: null }],
      config,
    )
    expect(tree.renderMarkdown()).toBe(
      '```\n── Root\n```',
    )
  })
})

// ── builder() ────────────────────────────────────────────────────────────

describe('AsciiTree.builder', () => {
  it('builds a single root', () => {
    const tree = AsciiTree.builder()
      .addRoot('a', 'Root')
      .build()

    expect(tree.size).toBe(1)
    expect(tree.render()).toBe('── Root')
  })

  it('builds root with detail', () => {
    const tree = AsciiTree.builder()
      .addRoot('a', 'Root', 'detail line')
      .build()

    expect(tree.render()).toBe(
      '── Root\n' +
      '   detail line',
    )
  })

  it('builds nested hierarchy', () => {
    const tree = AsciiTree.builder()
      .addRoot('a', 'Root')
      .addChild('a', 'b', 'Child A')
      .addChild('a', 'c', 'Child B')
      .addChild('b', 'd', 'Grandchild')
      .build()

    expect(tree.size).toBe(4)
    expect(tree.render()).toBe(
      '── Root\n' +
      '   ├── Child A\n' +
      '   │   └── Grandchild\n' +
      '   └── Child B',
    )
  })

  it('throws when adding child to non-existent parent', () => {
    expect(() => {
      AsciiTree.builder()
        .addChild('missing', 'a', 'Orphan')
    }).toThrow('parent "missing" not found')
  })

  it('supports chaining', () => {
    const builder = AsciiTree.builder()
    const result = builder.addRoot('a', 'R')
    expect(result).toBe(builder)
    const result2 = builder.addChild('a', 'b', 'C')
    expect(result2).toBe(builder)
  })
})

// ── Pipe alignment (visual regression) ───────────────────────────────────

describe('AsciiTree pipe alignment', () => {
  it('correctly aligns pipes for non-last parent with children', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'A', parentId: null },
        { id: 'a1', name: 'A1', parentId: 'a' },
        { id: 'a2', name: 'A2', parentId: 'a' },
        { id: 'b', name: 'B', parentId: null },
        { id: 'b1', name: 'B1', parentId: 'b' },
      ],
      config,
    )
    // The │ pipe under A must continue while A has a sibling B
    expect(tree.render()).toBe(
      '├── A\n' +
      '│   ├── A1\n' +
      '│   └── A2\n' +
      '└── B\n' +
      '    └── B1',
    )
  })

  it('mixes detail with children correctly', () => {
    const tree = AsciiTree.from(
      [
        { id: 'a', name: 'A', parentId: null, detail: 'a-detail' },
        { id: 'a1', name: 'A1', parentId: 'a', detail: 'a1-detail' },
        { id: 'b', name: 'B', parentId: null },
      ],
      config,
    )
    expect(tree.render()).toBe(
      '├── A\n' +
      '│   a-detail\n' +
      '│   └── A1\n' +
      '│       a1-detail\n' +
      '└── B',
    )
  })
})

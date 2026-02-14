import { describe, it, expect } from 'vitest'
import { parsePanel, resetIdCounter } from './parsePanel'
import type { PanelSection } from './parsePanel'

describe('parsePanel', () => {
  beforeEach(() => {
    resetIdCounter()
  })

  it('returns empty array for empty string', () => {
    expect(parsePanel('')).toEqual([])
    expect(parsePanel('   ')).toEqual([])
  })

  it('parses single H1 as root section', () => {
    const result = parsePanel('# Title')
    expect(result).toHaveLength(1)
    expect(result[0].depth).toBe(0)
    expect(result[0].title).toBe('Title')
    expect(result[0].children).toEqual([])
  })

  it('nests H2 under H1 and H3 under H2', () => {
    const md = [
      '# Outer',
      'Some text',
      '## Inner',
      '### Sub',
    ].join('\n')

    const result = parsePanel(md)
    expect(result).toHaveLength(1)

    const outer = result[0]
    expect(outer.title).toBe('Outer')
    expect(outer.depth).toBe(0)
    expect(outer.bodyMarkdown).toBe('Some text')
    expect(outer.children).toHaveLength(1)

    const inner = outer.children[0]
    expect(inner.title).toBe('Inner')
    expect(inner.depth).toBe(1)
    expect(inner.children).toHaveLength(1)

    const sub = inner.children[0]
    expect(sub.title).toBe('Sub')
    expect(sub.depth).toBe(2)
  })

  it('parses checkboxes', () => {
    const md = [
      '# Options',
      '- [ ] Unchecked item',
      '- [x] Checked item',
      '- [X] Also checked',
    ].join('\n')

    const result = parsePanel(md)
    const items = result[0].interactiveItems

    expect(items).toHaveLength(3)
    expect(items[0]).toMatchObject({ type: 'checkbox', label: 'Unchecked item', checked: false })
    expect(items[1]).toMatchObject({ type: 'checkbox', label: 'Checked item', checked: true })
    expect(items[2]).toMatchObject({ type: 'checkbox', label: 'Also checked', checked: true })
  })

  it('treats plain list items as body markdown, not interactive', () => {
    const md = [
      '# Pick one',
      '- Option A',
      '- Option B',
    ].join('\n')

    const result = parsePanel(md)
    expect(result[0].interactiveItems).toHaveLength(0)
    expect(result[0].bodyMarkdown).toContain('- Option A')
    expect(result[0].bodyMarkdown).toContain('- Option B')
  })

  it('preserves body markdown', () => {
    const md = [
      '# Section',
      'Regular paragraph text.',
      '',
      'Another paragraph.',
    ].join('\n')

    const result = parsePanel(md)
    expect(result[0].bodyMarkdown).toBe('Regular paragraph text.\n\nAnother paragraph.')
  })

  it('treats H2 without preceding H1 as root-level', () => {
    const md = [
      '## Standalone Inner',
      '- item',
    ].join('\n')

    const result = parsePanel(md)
    expect(result).toHaveLength(1)
    expect(result[0].depth).toBe(1)
    expect(result[0].title).toBe('Standalone Inner')
    expect(result[0].bodyMarkdown).toBe('- item')
  })

  it('handles multiple root sections', () => {
    const md = [
      '# First',
      '# Second',
      '# Third',
    ].join('\n')

    const result = parsePanel(md)
    expect(result).toHaveLength(3)
  })

  it('handles content before any heading', () => {
    const md = [
      'Some preamble text',
      '# Actual Section',
    ].join('\n')

    const result = parsePanel(md)
    expect(result).toHaveLength(2)
    expect(result[0].title).toBe('')
    expect(result[0].bodyMarkdown).toBe('Some preamble text')
  })

  it('assigns unique ids to all elements', () => {
    const md = [
      '# Section',
      '- [ ] Check A',
      '- [ ] Check B',
      '## Child',
    ].join('\n')

    const result = parsePanel(md)
    const ids = new Set<string>()

    const collectIds = (s: PanelSection) => {
      ids.add(s.id)
      for (const item of s.interactiveItems) ids.add(item.id)
      for (const child of s.children) collectIds(child)
    }

    for (const section of result) collectIds(section)
    // Section + 2 checkboxes + child section = 4 unique ids
    expect(ids.size).toBe(4)
  })
})

import { describe, it, expect, beforeEach } from 'vitest'
import { extractContent } from './contentExtraction'
import { createChipElement, ZERO_WIDTH_SPACE } from './chipInsertion'
import type { MentionToken } from '@/stores/contextMentionStore'
import type { PickableEntity } from '@/stores/contextMentionStore'

const makeToken = (id: string, label: string, kind: PickableEntity['kind'] = 'context-node'): MentionToken => ({
  id,
  entityId: `entity-${id}`,
  kind,
  label,
  color: '#10b981',
  chipKey: null,
  chipPreview: null,
  entity: {
    kind,
    id: `entity-${id}`,
    name: label,
    summary: `${kind}: ${label}`,
    data: { content: `content of ${label}` },
  } satisfies PickableEntity,
})

describe('extractContent', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    container = document.createElement('div')
  })

  it('extracts plain text', () => {
    container.textContent = 'Hello world'
    expect(extractContent(container, [])).toBe('Hello world')
  })

  it('strips zero-width spaces from text nodes', () => {
    container.textContent = `${ZERO_WIDTH_SPACE}Hello${ZERO_WIDTH_SPACE}`
    expect(extractContent(container, [])).toBe('Hello')
  })

  it('extracts text with BR as newline', () => {
    container.appendChild(document.createTextNode('Line 1'))
    container.appendChild(document.createElement('br'))
    container.appendChild(document.createTextNode('Line 2'))

    expect(extractContent(container, [])).toBe('Line 1\nLine 2')
  })

  it('handles DIV-wrapped lines (browser newline behavior)', () => {
    const div1 = document.createElement('div')
    div1.textContent = 'Line 1'
    const div2 = document.createElement('div')
    div2.textContent = 'Line 2'
    container.appendChild(div1)
    container.appendChild(div2)

    expect(extractContent(container, [])).toBe('Line 1\nLine 2')
  })

  it('does not add leading newline for first DIV', () => {
    const div = document.createElement('div')
    div.textContent = 'Only line'
    container.appendChild(div)

    expect(extractContent(container, [])).toBe('Only line')
  })

  it('expands mention chips to formatted context', () => {
    const token = makeToken('t1', 'My Context')
    const chip = createChipElement(token)

    container.appendChild(document.createTextNode('Before '))
    container.appendChild(chip)
    container.appendChild(document.createTextNode(ZERO_WIDTH_SPACE))
    container.appendChild(document.createTextNode(' After'))

    const result = extractContent(container, [token])
    expect(result).toContain('Before ')
    expect(result).toContain('[Context: Context Node] My Context')
    expect(result).toContain('content of My Context')
    expect(result).toContain(' After')
  })

  it('handles multiple chips interleaved with text', () => {
    const t1 = makeToken('t1', 'First')
    const t2 = makeToken('t2', 'Second')

    container.appendChild(document.createTextNode('Start '))
    container.appendChild(createChipElement(t1))
    container.appendChild(document.createTextNode(`${ZERO_WIDTH_SPACE} middle `))
    container.appendChild(createChipElement(t2))
    container.appendChild(document.createTextNode(`${ZERO_WIDTH_SPACE} end`))

    const result = extractContent(container, [t1, t2])
    expect(result).toContain('Start ')
    expect(result).toContain('[Context: Context Node] First')
    expect(result).toContain(' middle ')
    expect(result).toContain('[Context: Context Node] Second')
    expect(result).toContain(' end')
  })

  it('skips chips with no matching token', () => {
    const token = makeToken('t1', 'Known')
    const unknownChip = createChipElement(makeToken('unknown', 'Missing'))

    container.appendChild(createChipElement(token))
    container.appendChild(unknownChip)

    const result = extractContent(container, [token])
    expect(result).toContain('[Context: Context Node] Known')
    expect(result).not.toContain('Missing')
  })

  it('expands shared-field chips to compact format', () => {
    const token: MentionToken = {
      id: 't-sf',
      entityId: 'entity-sf',
      kind: 'shared-field',
      label: 'Test Node',
      color: '#10b981',
      chipKey: 'name',
      chipPreview: 'Test Node',
      entity: {
        kind: 'shared-field',
        id: 'entity-sf',
        name: 'Test Node',
        summary: 'shared-field: Test Node',
        data: { fieldType: 'Name', value: 'Test Node' },
      },
    }
    const chip = createChipElement(token)
    container.appendChild(chip)

    const result = extractContent(container, [token])
    expect(result).toBe('Name: Test Node')
    expect(result).not.toContain('[Context:')
  })

  it('returns empty string for empty container', () => {
    expect(extractContent(container, [])).toBe('')
  })
})

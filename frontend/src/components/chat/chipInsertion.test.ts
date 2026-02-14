import { describe, it, expect, beforeEach } from 'vitest'
import { createChipElement, insertChipAtCursor, removeChipFromDOM, ZERO_WIDTH_SPACE } from './chipInsertion'
import type { MentionToken } from '@/stores/contextMentionStore'
import type { PickableEntity } from '@/stores/contextMentionStore'

const makeToken = (id: string, label: string, color = '#10b981'): MentionToken => ({
  id,
  entityId: `entity-${id}`,
  kind: 'context-node',
  label,
  color,
  chipKey: null,
  chipPreview: null,
  entity: {
    kind: 'context-node',
    id: `entity-${id}`,
    name: label,
    summary: `Context: ${label}`,
    data: { content: `content of ${label}` },
  } satisfies PickableEntity,
})

const makeShareToken = (id: string, chipKey: string, chipPreview: string, color = '#10b981'): MentionToken => ({
  id,
  entityId: `entity-${id}`,
  kind: 'shared-field',
  label: chipPreview,
  color,
  chipKey,
  chipPreview,
  entity: {
    kind: 'shared-field',
    id: `entity-${id}`,
    name: chipPreview,
    summary: `shared-field: ${chipPreview}`,
    data: { fieldType: chipKey, value: chipPreview },
  } satisfies PickableEntity,
})

describe('createChipElement', () => {
  it('creates a span with correct attributes', () => {
    const token = makeToken('t1', 'My Context')
    const chip = createChipElement(token)

    expect(chip.tagName).toBe('SPAN')
    expect(chip.getAttribute('contenteditable')).toBe('false')
    expect(chip.getAttribute('data-mention-id')).toBe('t1')
    expect(chip.getAttribute('data-entity-id')).toBe('entity-t1')
    expect(chip.getAttribute('data-mention-kind')).toBe('context-node')
    expect(chip.className).toBe('mention-chip')
  })

  it('contains a colored dot, label, and remove button', () => {
    const token = makeToken('t1', 'My Context', '#3b82f6')
    const chip = createChipElement(token)

    expect(chip.children).toHaveLength(3)

    const dot = chip.children[0] as HTMLSpanElement
    expect(dot.style.backgroundColor).toBe('rgb(59, 130, 246)')
    expect(dot.style.borderRadius).toBe('50%')

    const label = chip.children[1] as HTMLSpanElement
    expect(label.textContent).toBe('My Context')

    const removeBtn = chip.children[2] as HTMLSpanElement
    expect(removeBtn.textContent).toBe('\u00D7')
    expect(removeBtn.getAttribute('data-remove-mention')).toBe('t1')
  })

  it('renders two-part label for share tokens with chipKey and chipPreview', () => {
    const token = makeShareToken('t1', 'name', 'MyDocumenter')
    const chip = createChipElement(token)

    expect(chip.children).toHaveLength(4)

    const keySpan = chip.children[1] as HTMLSpanElement
    expect(keySpan.textContent).toBe('name: ')
    expect(keySpan.style.opacity).toBe('0.5')

    const valueSpan = chip.children[2] as HTMLSpanElement
    expect(valueSpan.textContent).toBe('"MyDocumenter"')
  })

  it('renders single label for tokens without chipKey', () => {
    const token = makeToken('t1', 'My Context')
    const chip = createChipElement(token)

    expect(chip.children).toHaveLength(3)
    const label = chip.children[1] as HTMLSpanElement
    expect(label.textContent).toBe('My Context')
  })

  it('applies color-based styling', () => {
    const token = makeToken('t1', 'Test', '#a78bfa')
    const chip = createChipElement(token)

    expect(chip.style.color).toBe('rgb(167, 139, 250)')
    expect(chip.style.border).toMatch(/a78bfa|167.*139.*250/)
  })
})

describe('insertChipAtCursor', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    container = document.createElement('div')
    container.setAttribute('contenteditable', 'true')
    document.body.appendChild(container)
  })

  it('appends chip to empty container', () => {
    const token = makeToken('t1', 'Context A')
    insertChipAtCursor(container, token)

    const chip = container.querySelector('[data-mention-id="t1"]')
    expect(chip).not.toBeNull()
    expect(chip?.textContent).toContain('Context A')
  })

  it('adds zero-width space after chip', () => {
    const token = makeToken('t1', 'Context A')
    insertChipAtCursor(container, token)

    const lastChild = container.lastChild
    expect(lastChild?.nodeType).toBe(Node.TEXT_NODE)
    expect(lastChild?.textContent).toBe(ZERO_WIDTH_SPACE)
  })

  it('appends multiple chips sequentially', () => {
    insertChipAtCursor(container, makeToken('t1', 'A'))
    insertChipAtCursor(container, makeToken('t2', 'B'))

    const chips = container.querySelectorAll('[data-mention-id]')
    expect(chips).toHaveLength(2)
  })
})

describe('removeChipFromDOM', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
  })

  it('removes chip and trailing zero-width space', () => {
    insertChipAtCursor(container, makeToken('t1', 'A'))
    expect(container.querySelector('[data-mention-id="t1"]')).not.toBeNull()

    removeChipFromDOM(container, 't1')
    expect(container.querySelector('[data-mention-id="t1"]')).toBeNull()
  })

  it('no-ops for non-existent mention id', () => {
    insertChipAtCursor(container, makeToken('t1', 'A'))
    removeChipFromDOM(container, 'nonexistent')
    expect(container.querySelector('[data-mention-id="t1"]')).not.toBeNull()
  })
})

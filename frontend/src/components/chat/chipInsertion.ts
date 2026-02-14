// ============================================================================
// chipInsertion — DOM creation and cursor-based insertion for mention chips
// ============================================================================

import type { MentionToken } from '@/stores/contextMentionStore'

const ZERO_WIDTH_SPACE = '\u200B'

const createChipElement = (token: MentionToken): HTMLSpanElement => {
  const chip = document.createElement('span')
  chip.setAttribute('contenteditable', 'false')
  chip.setAttribute('data-mention-id', token.id)
  chip.setAttribute('data-entity-id', token.entityId)
  chip.setAttribute('data-mention-kind', token.kind)
  chip.className = 'mention-chip'

  Object.assign(chip.style, {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    padding: '0 6px',
    margin: '0 2px',
    height: '20px',
    borderRadius: '4px',
    fontSize: '11px',
    fontWeight: '500',
    fontFamily: 'inherit',
    verticalAlign: 'baseline',
    userSelect: 'none',
    cursor: 'default',
    lineHeight: '20px',
    backgroundColor: `${token.color}1A`,
    border: `1px solid ${token.color}4D`,
    color: token.color,
  })

  const dot = document.createElement('span')
  Object.assign(dot.style, {
    width: '6px',
    height: '6px',
    borderRadius: '50%',
    backgroundColor: token.color,
    flexShrink: '0',
  })
  chip.appendChild(dot)

  if (token.chipKey && token.chipPreview) {
    const keySpan = document.createElement('span')
    keySpan.textContent = `${token.chipKey}: `
    Object.assign(keySpan.style, { opacity: '0.5' })
    chip.appendChild(keySpan)

    const valueSpan = document.createElement('span')
    valueSpan.textContent = `"${token.chipPreview}"`
    chip.appendChild(valueSpan)
  } else {
    const label = document.createElement('span')
    label.textContent = token.label
    chip.appendChild(label)
  }

  const removeBtn = document.createElement('span')
  removeBtn.textContent = '\u00D7'
  removeBtn.setAttribute('data-remove-mention', token.id)
  Object.assign(removeBtn.style, {
    cursor: 'pointer',
    marginLeft: '2px',
    fontSize: '13px',
    lineHeight: '1',
    opacity: '0.7',
  })
  chip.appendChild(removeBtn)

  return chip
}

const insertChipAtCursor = (container: HTMLDivElement, token: MentionToken): void => {
  const chip = createChipElement(token)
  const sel = window.getSelection()

  if (sel && sel.rangeCount > 0 && container.contains(sel.anchorNode)) {
    const range = sel.getRangeAt(0)
    range.deleteContents()
    range.insertNode(chip)

    const spacer = document.createTextNode(ZERO_WIDTH_SPACE)
    if (chip.nextSibling) {
      chip.parentNode!.insertBefore(spacer, chip.nextSibling)
    } else {
      chip.parentNode!.appendChild(spacer)
    }

    range.setStartAfter(spacer)
    range.collapse(true)
    sel.removeAllRanges()
    sel.addRange(range)
  } else {
    container.appendChild(chip)
    container.appendChild(document.createTextNode(ZERO_WIDTH_SPACE))
  }
}

const removeChipFromDOM = (container: HTMLDivElement, mentionId: string): void => {
  const chip = container.querySelector(`[data-mention-id="${CSS.escape(mentionId)}"]`)
  if (!chip) return

  const next = chip.nextSibling
  chip.remove()

  if (next?.nodeType === Node.TEXT_NODE && next.textContent === ZERO_WIDTH_SPACE) {
    next.remove()
  }
}

export { createChipElement, insertChipAtCursor, removeChipFromDOM, ZERO_WIDTH_SPACE }

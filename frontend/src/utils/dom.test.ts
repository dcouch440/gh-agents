import { describe, it, expect } from 'vitest'
import { isEditableTarget } from './dom'

describe('isEditableTarget', () => {
  it('returns false for null', () => {
    expect(isEditableTarget(null)).toBe(false)
  })

  it('returns false for a non-element event target', () => {
    expect(isEditableTarget(new EventTarget())).toBe(false)
  })

  it('returns true for an input', () => {
    expect(isEditableTarget(document.createElement('input'))).toBe(true)
  })

  it('returns true for a textarea', () => {
    expect(isEditableTarget(document.createElement('textarea'))).toBe(true)
  })

  it('returns true for a select', () => {
    expect(isEditableTarget(document.createElement('select'))).toBe(true)
  })

  it('returns true for a contenteditable element', () => {
    const div = document.createElement('div')
    // jsdom does not derive isContentEditable from the attribute.
    Object.defineProperty(div, 'isContentEditable', { value: true })
    expect(isEditableTarget(div)).toBe(true)
  })

  it('returns false for an ordinary element', () => {
    expect(isEditableTarget(document.createElement('div'))).toBe(false)
    expect(isEditableTarget(document.createElement('button'))).toBe(false)
    expect(isEditableTarget(document.body)).toBe(false)
  })
})

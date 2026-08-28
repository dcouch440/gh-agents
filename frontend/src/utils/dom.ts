/**
 * Whether a keyboard event's target is a field the user is typing into.
 *
 * Global (window/document) shortcut handlers must call this first: a listener
 * bound to `window` fires for every keystroke on the page, including those
 * meant for a chat box or form input elsewhere in the layout. Without the
 * guard, `Backspace` deletes canvas elements while the user is mid-sentence.
 *
 * `instanceof` narrows the `EventTarget` without a cast, and covers inputs
 * rendered by MUI (which forwards to a real `<input>`/`<textarea>`) as well as
 * contenteditable surfaces such as the rich chat composer.
 */
const isEditableTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  const tag = target.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

export { isEditableTarget }

/**
 * CodeMirror autocomplete extension for {variable.path} references
 *
 * Triggers when the user types `{` and offers completions from upstream
 * step output schemas. Uses a ref-based getter so the extension stays
 * stable while completions update reactively.
 */

import { autocompletion } from '@codemirror/autocomplete'
import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete'
import type { Extension } from '@codemirror/state'
import type { VariableCompletion } from './variableContext'

/**
 * Create a CodeMirror extension that provides `{`-triggered variable autocomplete.
 *
 * @param getCompletions - Getter that returns the latest variable completions.
 *   Use a ref-based pattern so the extension captures a stable function reference
 *   that always reads the latest data without requiring editor remount.
 */
const createVariableAutocomplete = (
  getCompletions: () => VariableCompletion[],
): Extension => {
  const completionSource = (ctx: CompletionContext): CompletionResult | null => {
    // Scan backward from cursor to find an opening `{`
    const line = ctx.state.doc.lineAt(ctx.pos)
    const textBefore = line.text.slice(0, ctx.pos - line.from)
    const braceIdx = textBefore.lastIndexOf('{')

    // No opening brace, or there's a closing brace after it — not in a variable
    if (braceIdx === -1) return null
    const afterBrace = textBefore.slice(braceIdx + 1)
    if (afterBrace.includes('}')) return null

    // Validate that the text after `{` looks like a partial variable path
    if (afterBrace.length > 0 && !/^[a-zA-Z_][a-zA-Z0-9_.]*$/.test(afterBrace)) return null

    const completions = getCompletions()
    if (completions.length === 0) return null

    // `from` is AFTER the `{` so CodeMirror filters against the typed path
    // (e.g., user types `{res` → filter text is `res`, matching `result.summary`)
    const from = line.from + braceIdx + 1

    // Check if there's already a closing `}` right after the cursor
    const charAfterCursor = line.text.charAt(ctx.pos - line.from)
    const to = charAfterCursor === '}' ? ctx.pos + 1 : ctx.pos

    // Build CodeMirror completion options
    const sectionCache = new Map<string, { name: string }>()

    const options = completions.map((c) => {
      let section = sectionCache.get(c.section)
      if (!section) {
        section = { name: c.section }
        sectionCache.set(c.section, section)
      }

      return {
        label: c.displayLabel,
        detail: c.detail,
        section,
        // Apply includes closing `}` — the `{` before `from` is preserved
        apply: `${c.displayLabel}}`,
      }
    })

    return {
      from,
      to,
      options,
      filter: true,
    }
  }

  return autocompletion({
    override: [completionSource],
    activateOnTyping: true,
  })
}

export { createVariableAutocomplete }

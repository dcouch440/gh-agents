import { createVariableAutocomplete } from './variableAutocomplete'
import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete'

// We test the completion logic by extracting the override function from the extension
// Since the extension is opaque, we mock autocompletion to capture the override

const { mockAutocompletion, capturedOverride } = vi.hoisted(() => {
  let captured: ((ctx: CompletionContext) => CompletionResult | null) | null = null
  return {
    mockAutocompletion: vi.fn((config: { override: Array<(ctx: CompletionContext) => CompletionResult | null> }) => {
      captured = config.override[0] ?? null
      return [] // Extension is opaque, return anything
    }),
    capturedOverride: () => captured,
  }
})

vi.mock('@codemirror/autocomplete', () => ({
  autocompletion: mockAutocompletion,
}))

const mockCompletions = [
  { displayLabel: 'result.summary', detail: 'string', section: 'Step A' },
  { displayLabel: 'result.items', detail: 'array', section: 'Step A' },
  { displayLabel: 'output.text', detail: 'string', section: 'Step B' },
]

const makeContext = (lineText: string, cursorOffset?: number): CompletionContext => {
  const pos = cursorOffset ?? lineText.length
  return {
    pos,
    state: {
      doc: {
        lineAt: () => ({ from: 0, text: lineText }),
      },
    },
  } as unknown as CompletionContext
}

describe('createVariableAutocomplete', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    createVariableAutocomplete(() => mockCompletions)
  })

  it('returns completions when cursor is after opening brace', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {'))
    expect(result).not.toBeNull()
    expect(result!.options).toHaveLength(3)
    expect(result!.from).toBe(7) // after the {
  })

  it('returns null when there is no opening brace', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello world'))
    expect(result).toBeNull()
  })

  it('returns null when brace is already closed', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {result.summary} more text', 30))
    expect(result).toBeNull()
  })

  it('returns null when completions list is empty', () => {
    createVariableAutocomplete(() => [])
    const source = capturedOverride()!
    const result = source(makeContext('Hello {'))
    expect(result).toBeNull()
  })

  it('includes closing brace in to when } exists after cursor', () => {
    const source = capturedOverride()!
    // Cursor at position 7 (after {), line has } at position 7
    const result = source(makeContext('Hello {}', 7))
    expect(result).not.toBeNull()
    expect(result!.to).toBe(8) // includes the }
  })

  it('filters partial variable paths', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {res'))
    expect(result).not.toBeNull()
    expect(result!.from).toBe(7) // after the {
  })

  it('returns null for invalid characters after brace', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {123'))
    expect(result).toBeNull()
  })

  it('groups options by section', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {'))
    expect(result).not.toBeNull()
    const sections = new Set(result!.options.map((o) => (o.section as { name: string }).name))
    expect(sections.size).toBe(2)
    expect(sections.has('Step A')).toBe(true)
    expect(sections.has('Step B')).toBe(true)
  })

  it('applies include closing brace in completion', () => {
    const source = capturedOverride()!
    const result = source(makeContext('Hello {'))
    expect(result).not.toBeNull()
    expect(result!.options[0]!.apply).toBe('result.summary}')
  })
})

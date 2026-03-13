import { describe, it, expect } from 'vitest'
import { buildDispatchSegments } from './traceSegments'
import type { DispatchTraceEvent } from '@/stores/dispatchStore'

describe('buildDispatchSegments', () => {
  it('returns empty array for empty trace', () => {
    expect(buildDispatchSegments([])).toEqual([])
  })

  it('merges consecutive tokens into a single text segment', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'Hello ', ts: '2025-01-01T00:00:00Z' },
      { type: 'token', content: 'world', ts: '2025-01-01T00:00:01Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toEqual([{ type: 'text', content: 'Hello world' }])
  })

  it('produces text → tool → text sequence', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'Let me search.', ts: '2025-01-01T00:00:00Z' },
      { type: 'tool_start', toolName: 'web_search', toolId: 't1', input: { query: 'test' }, ts: '2025-01-01T00:00:01Z' },
      { type: 'tool_end', toolName: 'web_search', toolId: 't1', result: { results: [] }, ts: '2025-01-01T00:00:02Z' },
      { type: 'token', content: 'Done searching.', ts: '2025-01-01T00:00:03Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toHaveLength(3)
    expect(segments[0]).toEqual({ type: 'text', content: 'Let me search.' })
    expect(segments[1]).toEqual({
      type: 'tool',
      toolId: 't1',
      toolName: 'web_search',
      input: { query: 'test' },
      result: { results: [] },
      status: 'complete',
    })
    expect(segments[2]).toEqual({ type: 'text', content: 'Done searching.' })
  })

  it('marks unfinished tool as running with null result', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'tool_start', toolName: 'update_prompt', toolId: 't1', input: { prompt: 'new' }, ts: '2025-01-01T00:00:00Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toEqual([
      {
        type: 'tool',
        toolId: 't1',
        toolName: 'update_prompt',
        input: { prompt: 'new' },
        result: null,
        status: 'running',
      },
    ])
  })

  it('handles error segments', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'Working...', ts: '2025-01-01T00:00:00Z' },
      { type: 'error', error: 'Rate limit exceeded', ts: '2025-01-01T00:00:01Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toEqual([
      { type: 'text', content: 'Working...' },
      { type: 'error', error: 'Rate limit exceeded' },
    ])
  })

  it('handles interleaved tools and tokens', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'A', ts: '2025-01-01T00:00:00Z' },
      { type: 'tool_start', toolName: 'read_context', toolId: 't1', input: {}, ts: '2025-01-01T00:00:01Z' },
      { type: 'tool_end', toolName: 'read_context', toolId: 't1', result: 'ctx', ts: '2025-01-01T00:00:02Z' },
      { type: 'token', content: 'B', ts: '2025-01-01T00:00:03Z' },
      { type: 'tool_start', toolName: 'update_prompt', toolId: 't2', input: { p: 1 }, ts: '2025-01-01T00:00:04Z' },
      { type: 'tool_end', toolName: 'update_prompt', toolId: 't2', result: 'ok', ts: '2025-01-01T00:00:05Z' },
      { type: 'token', content: 'C', ts: '2025-01-01T00:00:06Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toHaveLength(5)
    expect(segments[0]).toEqual({ type: 'text', content: 'A' })
    expect(segments[1]).toMatchObject({ type: 'tool', toolId: 't1', status: 'complete' })
    expect(segments[2]).toEqual({ type: 'text', content: 'B' })
    expect(segments[3]).toMatchObject({ type: 'tool', toolId: 't2', status: 'complete' })
    expect(segments[4]).toEqual({ type: 'text', content: 'C' })
  })

  it('handles tool_end without matching tool_start gracefully', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'tool_end', toolName: 'unknown', toolId: 'orphan', result: 'data', ts: '2025-01-01T00:00:00Z' },
    ]
    // Should not crash — just ignores the orphan tool_end
    const segments = buildDispatchSegments(trace)
    expect(segments).toEqual([])
  })

  it('produces phase segment from phase_marker event', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'Builder done.', ts: '2025-01-01T00:00:00Z' },
      { type: 'phase_marker', label: 'Designer phase: configuring 3 agent(s)...', ts: '2025-01-01T00:00:01Z' },
      { type: 'tool_start', toolName: 'write_file', toolId: 't1', input: { path: 'design/agents/scanner.json' }, ts: '2025-01-01T00:00:02Z' },
      { type: 'tool_end', toolName: 'write_file', toolId: 't1', result: { status: 'written' }, ts: '2025-01-01T00:00:03Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toHaveLength(3)
    expect(segments[0]).toEqual({ type: 'text', content: 'Builder done.' })
    expect(segments[1]).toEqual({ type: 'phase', label: 'Designer phase: configuring 3 agent(s)...' })
    expect(segments[2]).toMatchObject({ type: 'tool', toolName: 'write_file', status: 'complete' })
  })

  it('flushes text buffer before phase segment', () => {
    const trace: DispatchTraceEvent[] = [
      { type: 'token', content: 'thinking...', ts: '2025-01-01T00:00:00Z' },
      { type: 'phase_marker', label: 'Phase 2', ts: '2025-01-01T00:00:01Z' },
    ]
    const segments = buildDispatchSegments(trace)
    expect(segments).toEqual([
      { type: 'text', content: 'thinking...' },
      { type: 'phase', label: 'Phase 2' },
    ])
  })
})

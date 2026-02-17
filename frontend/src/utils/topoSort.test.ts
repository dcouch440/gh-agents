import { topoSortStepIds } from './topoSort'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

const makeStep = (id: string, order: number, mode = 'single'): WorkflowStep =>
  ({
    id,
    display_order: order,
    execution_mode: mode,
  }) as WorkflowStep

const makeEdge = (from: string, to: string): WorkflowStepEdge =>
  ({
    id: `${from}-${to}`,
    from_step_id: from,
    to_step_id: to,
  }) as WorkflowStepEdge

describe('topoSortStepIds', () => {
  it('returns empty array for empty input', () => {
    expect(topoSortStepIds([], [])).toEqual([])
  })

  it('returns single step', () => {
    const steps = [makeStep('a', 0)]
    expect(topoSortStepIds(steps, [])).toEqual(['a'])
  })

  it('sorts a linear chain A → B → C', () => {
    const steps = [makeStep('c', 2), makeStep('a', 0), makeStep('b', 1)]
    const edges = [makeEdge('a', 'b'), makeEdge('b', 'c')]
    expect(topoSortStepIds(steps, edges)).toEqual(['a', 'b', 'c'])
  })

  it('uses display_order as tiebreaker for parallel branches', () => {
    // Root → B (order 2) and Root → A (order 1) — A should come first
    const steps = [
      makeStep('root', 0),
      makeStep('b', 2),
      makeStep('a', 1),
    ]
    const edges = [makeEdge('root', 'a'), makeEdge('root', 'b')]
    const result = topoSortStepIds(steps, edges)
    expect(result[0]).toBe('root')
    expect(result.indexOf('a')).toBeLessThan(result.indexOf('b'))
  })

  it('filters out context and input steps by default', () => {
    const steps = [
      makeStep('ctx', 0, 'context'),
      makeStep('inp', 1, 'input'),
      makeStep('main', 2, 'single'),
    ]
    const edges = [makeEdge('ctx', 'main'), makeEdge('inp', 'main')]
    const result = topoSortStepIds(steps, edges)
    expect(result).toEqual(['main'])
  })

  it('includes all steps when includeAll is true', () => {
    const steps = [
      makeStep('ctx', 0, 'context'),
      makeStep('inp', 1, 'input'),
      makeStep('main', 2, 'single'),
    ]
    const edges = [makeEdge('ctx', 'main'), makeEdge('inp', 'main')]
    const result = topoSortStepIds(steps, edges, { includeAll: true })
    expect(result).toHaveLength(3)
    expect(result).toContain('ctx')
    expect(result).toContain('inp')
    expect(result).toContain('main')
    // ctx and inp come before main (they're sources)
    expect(result.indexOf('main')).toBe(2)
  })

  it('appends cycle remnants by display_order', () => {
    // A → B → C → B (cycle between B and C), D is independent
    const steps = [
      makeStep('a', 0),
      makeStep('b', 1),
      makeStep('c', 2),
    ]
    const edges = [makeEdge('a', 'b'), makeEdge('b', 'c'), makeEdge('c', 'b')]
    const result = topoSortStepIds(steps, edges)
    // A is the only source, B and C form a cycle
    expect(result[0]).toBe('a')
    expect(result).toHaveLength(3)
    expect(result).toContain('b')
    expect(result).toContain('c')
  })

  it('handles diamond dependency correctly', () => {
    //   A
    //  / \
    // B   C
    //  \ /
    //   D
    const steps = [
      makeStep('a', 0),
      makeStep('b', 1),
      makeStep('c', 2),
      makeStep('d', 3),
    ]
    const edges = [
      makeEdge('a', 'b'),
      makeEdge('a', 'c'),
      makeEdge('b', 'd'),
      makeEdge('c', 'd'),
    ]
    const result = topoSortStepIds(steps, edges)
    expect(result[0]).toBe('a')
    expect(result[result.length - 1]).toBe('d')
    expect(result.indexOf('b')).toBeLessThan(result.indexOf('d'))
    expect(result.indexOf('c')).toBeLessThan(result.indexOf('d'))
  })

  it('ignores edges to steps outside the navigable set', () => {
    const steps = [
      makeStep('ctx', 0, 'context'),
      makeStep('a', 1),
    ]
    const edges = [makeEdge('ctx', 'a')]
    // Without includeAll, ctx is filtered out, so the edge is ignored
    const result = topoSortStepIds(steps, edges)
    expect(result).toEqual(['a'])
  })
})

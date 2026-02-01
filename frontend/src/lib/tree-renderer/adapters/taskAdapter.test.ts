import { describe, it, expect } from 'vitest'
import { tasksToTree } from './taskAdapter'
import type { Task } from '@/types'

const createMockTask = (overrides: Partial<Task>): Task => ({
  id: 'task-001',
  slice_id: null,
  title: 'Test task',
  description: 'A task for testing',
  assigned_tier: 'worker',
  assigned_agent: null,
  status: 'pending',
  priority: 'normal',
  context_files: [],
  metadata: null,
  depends_on: [],
  retry_count: 0,
  max_retries: 3,
  last_error: null,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
  ...overrides,
})

describe('tasksToTree', () => {
  it('converts empty tasks array to empty tree', () => {
    const result = tasksToTree([])
    expect(result.nodes).toEqual({})
    expect(result.rootIds).toEqual([])
    expect(result.edges).toEqual([])
  })

  it('converts single task to tree with one root', () => {
    const task = createMockTask({ id: 'task-001', title: 'First task' })
    const result = tasksToTree([task])

    expect(result.rootIds).toEqual(['task-001'])
    expect(result.nodes['task-001']).toMatchObject({
      id: 'task-001',
      label: 'First task',
      status: 'pending',
      children: [],
      metadata: {
        priority: 'normal',
        assignedAgent: null,
        retryCount: 0,
        lastError: null,
      },
    })
    expect(result.edges).toEqual([])
  })

  it('converts task statuses to node statuses', () => {
    const tasks = [
      createMockTask({ id: 'task-1', status: 'pending' }),
      createMockTask({ id: 'task-2', status: 'in_progress' }),
      createMockTask({ id: 'task-3', status: 'review' }),
      createMockTask({ id: 'task-4', status: 'completed' }),
      createMockTask({ id: 'task-5', status: 'failed' }),
    ]
    const result = tasksToTree(tasks)

    expect(result.nodes['task-1']?.status).toBe('pending')
    expect(result.nodes['task-2']?.status).toBe('running')
    expect(result.nodes['task-3']?.status).toBe('waiting')
    expect(result.nodes['task-4']?.status).toBe('completed')
    expect(result.nodes['task-5']?.status).toBe('failed')
  })

  it('creates dependency edges and parent-child relationships', () => {
    const tasks = [
      createMockTask({ id: 'task-1', title: 'Root', depends_on: [] }),
      createMockTask({ id: 'task-2', title: 'Child', depends_on: ['task-1'] }),
    ]
    const result = tasksToTree(tasks)

    expect(result.rootIds).toEqual(['task-1'])
    expect(result.nodes['task-1']?.children).toEqual(['task-2'])
    expect(result.nodes['task-2']?.children).toEqual([])
    expect(result.edges).toEqual([
      {
        sourceId: 'task-1',
        targetId: 'task-2',
        label: null,
        variant: 'dependency',
      },
    ])
  })

  it('handles multiple dependencies', () => {
    const tasks = [
      createMockTask({ id: 'task-1', depends_on: [] }),
      createMockTask({ id: 'task-2', depends_on: [] }),
      createMockTask({ id: 'task-3', depends_on: ['task-1', 'task-2'] }),
    ]
    const result = tasksToTree(tasks)

    expect(result.rootIds).toEqual(['task-1', 'task-2'])
    expect(result.nodes['task-1']?.children).toEqual(['task-3'])
    expect(result.nodes['task-2']?.children).toEqual(['task-3'])
    expect(result.nodes['task-3']?.children).toEqual([])
    expect(result.edges).toHaveLength(2)
    expect(result.edges).toContainEqual({
      sourceId: 'task-1',
      targetId: 'task-3',
      label: null,
      variant: 'dependency',
    })
    expect(result.edges).toContainEqual({
      sourceId: 'task-2',
      targetId: 'task-3',
      label: null,
      variant: 'dependency',
    })
  })

  it('handles complex dependency tree', () => {
    const tasks = [
      createMockTask({ id: 'task-1', depends_on: [] }),
      createMockTask({ id: 'task-2', depends_on: ['task-1'] }),
      createMockTask({ id: 'task-3', depends_on: ['task-1'] }),
      createMockTask({ id: 'task-4', depends_on: ['task-2', 'task-3'] }),
    ]
    const result = tasksToTree(tasks)

    expect(result.rootIds).toEqual(['task-1'])
    expect(result.nodes['task-1']?.children).toContain('task-2')
    expect(result.nodes['task-1']?.children).toContain('task-3')
    expect(result.nodes['task-4']?.children).toEqual([])
    expect(result.edges).toHaveLength(4)
  })

  it('ignores dependencies to non-existent tasks', () => {
    const tasks = [
      createMockTask({ id: 'task-1', depends_on: ['nonexistent'] }),
    ]
    const result = tasksToTree(tasks)

    expect(result.rootIds).toEqual(['task-1'])
    expect(result.nodes['task-1']?.children).toEqual([])
    expect(result.edges).toEqual([])
  })

  it('preserves task metadata', () => {
    const task = createMockTask({
      id: 'task-1',
      priority: 'high',
      assigned_agent: 'agent-001',
      retry_count: 2,
      last_error: 'Connection timeout',
    })
    const result = tasksToTree([task])

    expect(result.nodes['task-1']?.metadata).toEqual({
      priority: 'high',
      assignedAgent: 'agent-001',
      retryCount: 2,
      lastError: 'Connection timeout',
    })
  })

  it('handles multiple root tasks', () => {
    const tasks = [
      createMockTask({ id: 'task-1', depends_on: [] }),
      createMockTask({ id: 'task-2', depends_on: [] }),
      createMockTask({ id: 'task-3', depends_on: [] }),
    ]
    const result = tasksToTree(tasks)

    expect(result.rootIds).toEqual(['task-1', 'task-2', 'task-3'])
    expect(result.edges).toEqual([])
  })
})

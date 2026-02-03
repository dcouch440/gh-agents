import type { Task } from '@/types'
import type { TreeData, TreeNode, TreeEdgeData, NodeStatus } from '../types'

type TaskMeta = {
  priority: string
  assignedAgent: string | null
  retryCount: number
  lastError: string | null
}

const taskStatusToNodeStatus = (status: string): NodeStatus => {
  if (status === 'in_progress') return 'running'
  if (status === 'review') return 'waiting'
  if (status === 'completed') return 'completed'
  if (status === 'failed') return 'failed'
  return 'pending'
}

const tasksToTree = (tasks: Task[]): TreeData<TaskMeta> => {
  const taskMap = new Map(tasks.map((t) => [t.id, t]))
  const nodes: Record<string, TreeNode<TaskMeta>> = {}
  const edges: TreeEdgeData[] = []
  const hasParent = new Set<string>()

  // Build children from depends_on (reverse: dependency → dependant)
  const childrenMap = new Map<string, string[]>()
  for (const task of tasks) {
    for (const depId of task.depends_on) {
      if (!taskMap.has(depId)) continue
      const existing = childrenMap.get(depId) ?? []
      existing.push(task.id)
      childrenMap.set(depId, existing)
      hasParent.add(task.id)

      edges.push({
        sourceId: depId,
        targetId: task.id,
        label: null,
        variant: 'dependency',
      })
    }
  }

  for (const task of tasks) {
    nodes[task.id] = {
      id: task.id,
      label: task.title,
      status: taskStatusToNodeStatus(task.status),
      children: childrenMap.get(task.id) ?? [],
      metadata: {
        priority: task.priority,
        assignedAgent: task.assigned_agent,
        retryCount: task.retry_count,
        lastError: task.last_error,
      },
    }
  }

  const rootIds = tasks.filter((t) => !hasParent.has(t.id)).map((t) => t.id)

  return { nodes, rootIds, edges }
}

export { tasksToTree }
export type { TaskMeta }

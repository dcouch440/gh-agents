// ============================================================================
// taskStore — Task CRUD Store
// ============================================================================

import { createResourceStore } from './lib'
import { api } from '@/api'
import type { Task, CreateTaskRequest } from '@/types/task'

const taskStore = createResourceStore<Task, CreateTaskRequest, Partial<Task>>({
  name: 'tasks',
  api: {
    list: () => api.tasks.list(),
    get: (id) => api.tasks.get(id),
    create: (body) => api.tasks.create(body),
    update: (id, body) => api.tasks.update(id, body),
    delete: (id) => api.tasks.delete(id),
  },
  unwrapList: (res) => (res as { items: Task[] }).items,
})

export { taskStore }

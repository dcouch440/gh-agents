// ============================================================================
// toolStore — Tool CRUD Store
// ============================================================================

import { createResourceStore } from './lib'
import { api } from '@/api'
import type { Tool, CreateToolRequest, UpdateToolRequest } from '@/types/tool'

const toolStore = createResourceStore<Tool, CreateToolRequest, UpdateToolRequest>({
  name: 'tools',
  api: {
    list: () => api.tools.list(),
    get: (id) => api.tools.get(id),
    create: (body) => api.tools.create(body),
    update: (id, body) => api.tools.update(id, body),
    delete: (id) => api.tools.delete(id),
  },
})

export { toolStore }

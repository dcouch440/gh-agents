// ============================================================================
// outputSchemaStore — Output Schema CRUD Store
// ============================================================================

import { createResourceStore } from './lib'
import { api } from '@/api'
import type { OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest } from '@/types/schema'

const outputSchemaStore = createResourceStore<OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest>({
  name: 'outputSchemas',
  api: {
    list: () => api.outputSchemas.list(),
    get: (id) => api.outputSchemas.get(id),
    create: (body) => api.outputSchemas.create(body),
    update: (id, body) => api.outputSchemas.update(id, body),
    delete: (id) => api.outputSchemas.delete(id),
  },
  unwrapList: (res) => (res as { items: OutputSchema[] }).items,
})

export { outputSchemaStore }

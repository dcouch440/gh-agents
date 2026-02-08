// ============================================================================
// promptTemplateStore — Prompt Template CRUD Store
// ============================================================================

import { createResourceStore } from './lib'
import { api } from '@/api'
import type { PromptTemplate, CreatePromptTemplateRequest, UpdatePromptTemplateRequest } from '@/types/template'

const promptTemplateStore = createResourceStore<PromptTemplate, CreatePromptTemplateRequest, UpdatePromptTemplateRequest>({
  name: 'promptTemplates',
  api: {
    list: () => api.promptTemplates.list(),
    get: (id) => api.promptTemplates.get(id),
    create: (body) => api.promptTemplates.create(body),
    update: (id, body) => api.promptTemplates.update(id, body),
    delete: (id) => api.promptTemplates.delete(id),
  },
})

export { promptTemplateStore }

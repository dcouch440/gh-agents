// ============================================================================
// documentStore — Document CRUD Store
// ============================================================================

import { createResourceStore } from './lib'
import { nmFromArray } from './lib'
import { api } from '@/api'
import type { Document, CreateDocumentRequest, UpdateDocumentRequest } from '@/types/document'

const resourceStore = createResourceStore<Document, CreateDocumentRequest, UpdateDocumentRequest>({
  name: 'documents',
  api: {
    list: () => api.documents.list(),
    get: (id) => api.documents.get(id),
    create: (body) => api.documents.create(body),
    update: (id, body) => api.documents.update(id, body),
    delete: (id) => api.documents.delete(id),
  },
  unwrapList: (res) => (res as { items: Document[] }).items,
})

const search = async (query: string): Promise<void> => {
  resourceStore.store.setState({ loading: true, error: null })
  try {
    const response = await api.documents.search(query)
    const items = (response as { items: Document[] }).items
    resourceStore.store.setState({ items: nmFromArray(items), loading: false })
  } catch (e) {
    resourceStore.store.setState({
      loading: false,
      error: e instanceof Error ? e.message : 'Search failed',
    })
  }
}

const documentStore = { ...resourceStore, search }

export { documentStore }

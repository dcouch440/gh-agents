import { useState, useEffect, useCallback } from 'react'
import type { AgentContextResponse } from '@/types/agent'
import type { DocumentListItem } from '@/types/document'
import { api } from '@/api'

const useAgentDocuments = (agentId: string | null) => {
  const [documents, setDocuments] = useState<DocumentListItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const load = useCallback(async () => {
    if (!agentId) {
      setDocuments([])
      setLoading(false)
      return
    }

    setLoading(true)
    setError(null)
    try {
      const data: AgentContextResponse = await api.agents.getContext(agentId)
      setDocuments(data.documents)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load agent context')
    } finally {
      setLoading(false)
    }
  }, [agentId])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [load])

  const setContext = useCallback(
    async (documentIds: string[]) => {
      if (!agentId) return

      setSaving(true)
      setError(null)
      try {
        await api.agents.setContext(agentId, documentIds)
        await load()
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to update agent context')
      } finally {
        setSaving(false)
      }
    },
    [agentId, load]
  )

  const addDocument = useCallback(
    async (documentId: string) => {
      const currentIds = documents.map((d) => d.id)
      if (currentIds.includes(documentId)) return
      await setContext([...currentIds, documentId])
    },
    [documents, setContext]
  )

  const removeDocument = useCallback(
    async (documentId: string) => {
      const currentIds = documents.map((d) => d.id).filter((id) => id !== documentId)
      await setContext(currentIds)
    },
    [documents, setContext]
  )

  return {
    documents,
    loading,
    error,
    saving,
    reload: load,
    addDocument,
    removeDocument,
    setContext,
  }
}

export { useAgentDocuments }

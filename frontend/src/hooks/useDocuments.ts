import { useState, useEffect, useCallback } from 'react'
import type { Document } from '@/types/document'
import { api } from '@/api'

const useDocuments = () => {
  const [documents, setDocuments] = useState<Document[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.documents.list()
      setDocuments(data.items as unknown as Document[])
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load documents')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    void run()
    return () => { cancelled = true }
  }, [load])

  return { documents, loading, error, reload: load }
}

const useDocument = (id: string | null) => {
  const [document, setDocument] = useState<Document | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) {
      setDocument(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await api.documents.get(id)
        if (!cancelled) setDocument(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load document')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => { cancelled = true }
  }, [id])

  return { document, loading, error }
}

export { useDocuments, useDocument }

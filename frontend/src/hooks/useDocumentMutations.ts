import { useState, useCallback } from 'react'
import { api } from '@/api'
import type { Document, DocumentSearchResult, CreateDocumentRequest, UpdateDocumentRequest } from '@/types'

const useCreateDocument = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateDocumentRequest): Promise<Document> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Document>('/documents', body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create document'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useUpdateDocument = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateDocumentRequest): Promise<Document> => {
    setLoading(true)
    setError(null)
    try {
      return await api.patch<Document>(`/documents/${id}`, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update document'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useDeleteDocument = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(`/documents/${id}`)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete document'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useSearchDocuments = () => {
  const [results, setResults] = useState<DocumentSearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const search = useCallback(async (query: string): Promise<DocumentSearchResult[]> => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.get<DocumentSearchResult[]>(`/documents/search?q=${encodeURIComponent(query)}`)
      setResults(data)
      return data
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to search documents'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { results, search, loading, error }
}

export { useCreateDocument, useUpdateDocument, useDeleteDocument, useSearchDocuments }

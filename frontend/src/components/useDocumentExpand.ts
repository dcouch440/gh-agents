import { useState, useMemo } from 'react'
import { Collections } from '@/utils/collections'
import { api } from '@/api'
import type { DocumentListItem, Document } from '@/types/document'

type UseDocumentExpandResult = {
  expandedId: string | null
  loadingDocId: string | null
  toggleExpand: (docId: string, e: React.MouseEvent) => void
  getDocumentContent: (docId: string) => string
}

const useDocumentExpand = (documents: readonly DocumentListItem[]): UseDocumentExpandResult => {
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [loadedDocs, setLoadedDocs] = useState<Map<string, Document>>(new Map())
  const [loadingDocId, setLoadingDocId] = useState<string | null>(null)

  const documentsById = useMemo(() => Collections.indexById(documents as DocumentListItem[]), [documents])

  const toggleExpand = (docId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    const newExpandedId = expandedId === docId ? null : docId
    setExpandedId(newExpandedId)

    if (newExpandedId && !loadedDocs.has(newExpandedId)) {
      setLoadingDocId(newExpandedId)
      void api.documents
        .get(newExpandedId)
        .then((doc) => {
          setLoadedDocs((prev) => new Map(prev).set(newExpandedId, doc))
        })
        .catch((err: unknown) => {
          console.error('Failed to load document:', err)
        })
        .finally(() => {
          setLoadingDocId(null)
        })
    }
  }

  const getDocumentContent = (docId: string): string => {
    if (loadingDocId === docId) {
      return 'Loading document content...'
    }
    const fullDoc = loadedDocs.get(docId)
    if (fullDoc?.content) {
      return fullDoc.content
    }
    const listItem = documentsById.get(docId)
    return listItem?.summary ?? 'No content available'
  }

  return { expandedId, loadingDocId, toggleExpand, getDocumentContent }
}

export { useDocumentExpand }
export type { UseDocumentExpandResult }

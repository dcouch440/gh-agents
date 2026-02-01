type Document = {
  id: string
  user_id: string
  session_id: string | null
  title: string
  content: string
  summary: string
  doc_type: string
  ref_tag: string
  tags: string[]
  created_at: string
  updated_at: string
}

type DocumentSearchResult = {
  id: string
  title: string
  summary: string
  ref_tag: string
  snippet: string
}

type CreateDocumentRequest = {
  title: string
  content: string
  doc_type: string
  ref_tag?: string
  tags?: string[]
}

type UpdateDocumentRequest = Partial<CreateDocumentRequest>

export type { Document, DocumentSearchResult, CreateDocumentRequest, UpdateDocumentRequest }

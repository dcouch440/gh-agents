type DocumentNodeMode = 'entry' | 'document'

type DocumentNodeData = {
  label: string
  mode: DocumentNodeMode
  content: string
}

export type { DocumentNodeData, DocumentNodeMode }

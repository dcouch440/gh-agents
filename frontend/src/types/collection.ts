// ============================================================================
// Collection Types
// ============================================================================

export type Collection = {
  id: string
  user_id: string
  name: string
  description: string | null
  execution_mode: string
  created_at: string
  updated_at: string
}

export type CollectionRun = {
  id: string
  collection_id: string
  user_id: string
  status: string
  started_at: string
  completed_at: string | null
  error: string | null
}

export type CreateCollectionRequest = {
  name: string
  description?: string | null
  execution_mode?: string
}

export type UpdateCollectionRequest = {
  name?: string
  description?: string | null
  execution_mode?: string
}

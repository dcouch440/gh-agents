type OutputSchema = {
  id: string
  user_id: string
  name: string
  description: string | null
  json_schema: Record<string, unknown>
  created_at: string
  updated_at: string
}

type CreateOutputSchemaRequest = {
  name: string
  description?: string
  json_schema: Record<string, unknown>
}

type UpdateOutputSchemaRequest = Partial<CreateOutputSchemaRequest>

export type { OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest }

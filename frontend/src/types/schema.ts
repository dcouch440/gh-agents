type OutputSchema = {
  id: string
  name: string
  schema: Record<string, unknown>
  created_at: string
}

type CreateOutputSchemaRequest = {
  name: string
  schema: Record<string, unknown>
}

type UpdateOutputSchemaRequest = {
  name?: string
  schema?: Record<string, unknown>
}

export type { OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest }

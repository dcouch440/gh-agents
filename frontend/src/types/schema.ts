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

type UpdateOutputSchemaRequest = Partial<CreateOutputSchemaRequest>

export type { OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest }

type Tool = {
  id: string
  name: string
  description: string
  category: string
  parameter_schema: unknown
  output_schema: unknown
  enabled: boolean
  cluster_id: string | null
  is_builtin: boolean
}

export type { Tool }

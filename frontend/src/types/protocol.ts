// Protocol types — matches backend ProtocolResponse shape

type Protocol = {
  id: string
  name: string
  description: string
  protocol_type: string
  config: Record<string, unknown>
  version: number
  ports: ProtocolPort[]
  agent: ProtocolAgent | null
  output_schema: ProtocolSchema | null
  prompt_template: ProtocolTemplate | null
}

type ProtocolPort = {
  id: string
  port_name: string
  description: string
  agent_id: string
  display_order: number
}

type ProtocolAgent = {
  id: string
  name: string
  system_prompt: string
  model_provider: string
  model_id: string
}

type ProtocolSchema = {
  id: string
  name: string
  schema: Record<string, unknown>
}

type ProtocolTemplate = {
  id: string
  name: string
  content: string
}

type ProtocolTypeInfo = {
  name: string
  description: string
}

type CreateProtocolRequest = {
  name: string
  description?: string
  protocol_type: string
  config?: Record<string, unknown>
  agent_id?: string
  output_schema_id?: string
  prompt_template_id?: string
}

type UpdateProtocolRequest = {
  name?: string
  description?: string
  config?: Record<string, unknown>
  agent_id?: string
  output_schema_id?: string
  prompt_template_id?: string
}

type CreatePortRequest = {
  port_name: string
  description?: string
  agent_id: string
  display_order?: number
}

export type {
  Protocol,
  ProtocolPort,
  ProtocolAgent,
  ProtocolSchema,
  ProtocolTemplate,
  ProtocolTypeInfo,
  CreateProtocolRequest,
  UpdateProtocolRequest,
  CreatePortRequest,
}

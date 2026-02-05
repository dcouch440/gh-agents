type ToolRouter = {
  id: string
  user_id: string
  name: string
  description: string | null
  system_prompt: string
  model_id: string
  is_active: boolean
  parent_router_id: string | null
  level: number
  created_at: string
  updated_at: string
}

type CreateToolRouterRequest = {
  name: string
  description?: string
  system_prompt: string
  model_id: string
}

type UpdateToolRouterRequest = {
  name?: string
  description?: string
  system_prompt?: string
  model_id?: string
  is_active?: boolean
}

type SetRouterToolsRequest = {
  tool_ids: string[]
}

export type {
  ToolRouter,
  CreateToolRouterRequest,
  UpdateToolRouterRequest,
  SetRouterToolsRequest,
}

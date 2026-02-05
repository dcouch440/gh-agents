// Router Mode types for the router modes system
// Matches Rust RouterModeResponse structure from src/server/api/router_modes/mod.rs

/**
 * Router Mode response type
 * Represents a mode configuration for a tool router
 */
export type RouterMode = {
  id: string
  router_id: string
  mode_key: string
  display_name: string
  description: string
  system_prompt: string
  temperature: number
  max_tokens: number
  append_to_agent_system_prompt: boolean
  append_to_agent_tools: boolean
  display_order: number
  created_at: string
  updated_at: string
}

/**
 * Create router mode request
 * Required fields: mode_key, display_name, description, system_prompt
 * Optional fields have backend defaults
 */
export type CreateRouterModeRequest = {
  mode_key: string
  display_name: string
  description: string
  system_prompt: string
  temperature?: number
  max_tokens?: number
  append_to_agent_system_prompt?: boolean
  append_to_agent_tools?: boolean
  display_order?: number
}

/**
 * Update router mode request
 * All fields are optional (partial update)
 */
export type UpdateRouterModeRequest = Partial<CreateRouterModeRequest>

/**
 * Set mode tools request
 * Replaces all tools assigned to the mode
 */
export type SetModeToolsRequest = {
  tool_ids: string[]
}

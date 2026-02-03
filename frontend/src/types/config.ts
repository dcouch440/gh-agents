type ModelConfig = {
  provider: string
  model_id: string
  max_tokens: number
  temperature: number
}

type Config = {
  verbosity: string
  models: {
    orchestrator: ModelConfig
    worker: ModelConfig
    utility: ModelConfig
  }
  pool: {
    max_agents: number
  }
  autonomy: string
  git_strategy: string
  sandbox_mode: string
}

type UpdateModelConfig = {
  model_id?: string
  max_tokens?: number
  temperature?: number
}

type UpdateModelsRequest = {
  orchestrator?: UpdateModelConfig
  worker?: UpdateModelConfig
  utility?: UpdateModelConfig
}

type UpdatePoolRequest = {
  max_agents?: number
}

type UpdateConfigRequest = {
  verbosity?: string
  models?: UpdateModelsRequest
  pool?: UpdatePoolRequest
  autonomy?: string
  git_strategy?: string
  sandbox_mode?: string
}

export type { ModelConfig, Config, UpdateConfigRequest }

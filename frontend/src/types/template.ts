type PromptTemplate = {
  id: string
  user_id: string | null
  name: string
  description: string | null
  template: string
  variables: string[] | null
  created_at: string
  updated_at: string
}

type CreatePromptTemplateRequest = {
  name: string
  description?: string
  template: string
  variables?: string[]
}

type UpdatePromptTemplateRequest = Partial<CreatePromptTemplateRequest>

export type { PromptTemplate, CreatePromptTemplateRequest, UpdatePromptTemplateRequest }

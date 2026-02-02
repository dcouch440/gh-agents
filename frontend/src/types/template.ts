type PromptTemplate = {
  id: string
  name: string
  content: string
  created_at: string
}

type CreatePromptTemplateRequest = {
  name: string
  content: string
}

type UpdatePromptTemplateRequest = Partial<CreatePromptTemplateRequest>

export type { PromptTemplate, CreatePromptTemplateRequest, UpdatePromptTemplateRequest }

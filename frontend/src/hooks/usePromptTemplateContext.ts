import { useContext } from 'react'
import { PromptTemplateContext } from '@/contexts/PromptTemplateContext'

const usePromptTemplateContext = () => {
  const ctx = useContext(PromptTemplateContext)
  if (!ctx) throw new Error('usePromptTemplateContext must be used within PromptTemplateProvider')
  return ctx
}

export { usePromptTemplateContext }

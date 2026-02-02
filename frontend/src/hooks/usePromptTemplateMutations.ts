import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import type { PromptTemplate, CreatePromptTemplateRequest, UpdatePromptTemplateRequest } from '@/types/template'

const useCreatePromptTemplate = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreatePromptTemplateRequest): Promise<PromptTemplate> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<PromptTemplate>(API.PROMPT_TEMPLATES, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create prompt template'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useUpdatePromptTemplate = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdatePromptTemplateRequest): Promise<PromptTemplate> => {
    setLoading(true)
    setError(null)
    try {
      return await api.put<PromptTemplate>(API.PROMPT_TEMPLATE(id), body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update prompt template'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useDeletePromptTemplate = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.PROMPT_TEMPLATE(id))
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete prompt template'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useCreatePromptTemplate, useUpdatePromptTemplate, useDeletePromptTemplate }

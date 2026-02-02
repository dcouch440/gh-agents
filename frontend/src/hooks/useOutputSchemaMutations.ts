import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import { useOutputSchemaContext } from '@/hooks/useOutputSchemaContext'
import type { OutputSchema, CreateOutputSchemaRequest, UpdateOutputSchemaRequest } from '@/types/schema'

const useCreateOutputSchema = () => {
  const { reload } = useOutputSchemaContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateOutputSchemaRequest): Promise<OutputSchema> => {
    setLoading(true)
    setError(null)
    try {
      const schema = await api.post<OutputSchema>(API.OUTPUT_SCHEMAS, body)
      reload()
      return schema
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create output schema'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useUpdateOutputSchema = () => {
  const { reload } = useOutputSchemaContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateOutputSchemaRequest): Promise<OutputSchema> => {
    setLoading(true)
    setError(null)
    try {
      const schema = await api.put<OutputSchema>(API.OUTPUT_SCHEMA(id), body)
      reload()
      return schema
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update output schema'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useDeleteOutputSchema = () => {
  const { reload } = useOutputSchemaContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.OUTPUT_SCHEMA(id))
      reload()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete output schema'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

export { useCreateOutputSchema, useUpdateOutputSchema, useDeleteOutputSchema }

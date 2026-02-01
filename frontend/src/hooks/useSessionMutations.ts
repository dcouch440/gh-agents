import { useState, useCallback } from 'react'
import { api } from '../api'
import type { Session, CreateSessionRequest, UpdateSessionRequest } from '../types'

const useCreateSession = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateSessionRequest): Promise<Session> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Session>('/sessions', body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create session'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useUpdateSession = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateSessionRequest): Promise<Session> => {
    setLoading(true)
    setError(null)
    try {
      return await api.patch<Session>(`/sessions/${id}`, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update session'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useDeleteSession = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(`/sessions/${id}`)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete session'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useCreateSession, useUpdateSession, useDeleteSession }

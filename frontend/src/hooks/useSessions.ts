import { useState, useEffect, useCallback } from 'react'
import type { Session, ChatMessage, Mode } from '@/types/session'
import { USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const useSessions = () => {
  const [sessions, setSessions] = useState<Session[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getSessions()
        : await api.get<Session[]>('/sessions')
      setSessions(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load sessions')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    run()
    return () => { cancelled = true }
  }, [load])

  return { sessions, loading, error, reload: load }
}

const useChatHistory = (sessionId: string | null) => {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!sessionId) {
      setMessages([])
      setLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = USE_MOCK_DATA
          ? await mock.getChatHistory(sessionId)
          : await api.get<ChatMessage[]>(`/sessions/${sessionId}/history`)
        if (!cancelled) setMessages(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load chat history')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [sessionId])

  return { messages, loading, error }
}

const useModes = () => {
  const [modes, setModes] = useState<Mode[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = USE_MOCK_DATA
          ? await mock.getModes()
          : await api.get<Mode[]>('/modes')
        if (!cancelled) setModes(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load modes')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [])

  return { modes, loading, error }
}

export { useSessions, useChatHistory, useModes }

import { useState, useEffect, useCallback } from 'react'
import type { Session, ChatMessage, Mode } from '@/types/session'
import { api } from '@/api'

const useSessions = () => {
  const [sessions, setSessions] = useState<Session[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.sessions.list()
      setSessions(data.items)
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
    void run()
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
        const data = await api.sessions.getHistory(sessionId)
        if (!cancelled) setMessages(data.messages)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load chat history')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
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
        const data = await api.modes.list() as Mode[]
        if (!cancelled) setModes(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load modes')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => { cancelled = true }
  }, [])

  return { modes, loading, error }
}

export { useSessions, useChatHistory, useModes }

import { useState, useEffect, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'
import type { ChatMessageData } from '@/components/chat'

type UseStepDebugLogResult = {
  messages: ChatMessageData[]
  isLoading: boolean
  error: string | null
  refresh: () => void
}

const useStepDebugLog = (stepId: string): UseStepDebugLogResult => {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [messages, setMessages] = useState<ChatMessageData[]>([])

  const fetchDebugData = useCallback(async () => {
    if (!workflowId) return

    setIsLoading(true)
    setError(null)
    try {
      const data = await api.workflows.getStepChatDebug(workflowId, stepId)

      const debugMessages: ChatMessageData[] = [
        {
          id: 'system-prompt',
          role: 'system',
          content: data.system_prompt,
        },
        ...data.messages.map((m, idx) => ({
          id: `debug-msg-${idx}`,
          role: m.role as 'user' | 'assistant',
          content: m.content,
        })),
      ]

      setMessages(debugMessages)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load debug data')
    } finally {
      setIsLoading(false)
    }
  }, [workflowId, stepId])

  useEffect(() => {
    void fetchDebugData()
  }, [fetchDebugData])

  const refresh = useCallback(() => {
    void fetchDebugData()
  }, [fetchDebugData])

  return { messages, isLoading, error, refresh }
}

export { useStepDebugLog }
export type { UseStepDebugLogResult }

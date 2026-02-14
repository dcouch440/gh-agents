import { useState, useEffect, useCallback } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import IconButton from '@mui/material/IconButton'
import Tooltip from '@mui/material/Tooltip'
import CircularProgress from '@mui/material/CircularProgress'
import RefreshOutlined from '@mui/icons-material/RefreshOutlined'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'
import { MessageList } from '@/components/chat'
import type { ChatMessageData } from '@/components/chat'

type DebugLogTabProps = {
  stepId: string
}

function DebugLogTab({ stepId }: DebugLogTabProps) {
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
      const is404 = e instanceof Error && e.message.includes('404')
      if (is404) {
        // No session yet — show system prompt only
        try {
          const data = await api.workflows.getStepChatDebug(workflowId, stepId)
          setMessages([{
            id: 'system-prompt',
            role: 'system',
            content: data.system_prompt,
          }])
        } catch {
          setError(e instanceof Error ? e.message : 'Failed to load debug data')
        }
      } else {
        setError(e instanceof Error ? e.message : 'Failed to load debug data')
      }
    } finally {
      setIsLoading(false)
    }
  }, [workflowId, stepId])

  useEffect(() => {
    void fetchDebugData()
  }, [fetchDebugData])

  if (!workflowId) return null

  if (isLoading && messages.length === 0) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 1.5,
          py: 0.5,
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography variant="body2" sx={{ fontWeight: 500, color: 'text.secondary' }}>
          Debug
        </Typography>
        <Tooltip title="Refresh">
          <span>
            <IconButton size="small" onClick={() => void fetchDebugData()} disabled={isLoading}>
              <RefreshOutlined fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </Box>

      {error ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
          <Typography variant="body2" color="error">
            {error}
          </Typography>
        </Box>
      ) : (
        <MessageList
          messages={messages}
          emptyMessage="No debug data available yet."
        />
      )}
    </Box>
  )
}

export { DebugLogTab }
export type { DebugLogTabProps }

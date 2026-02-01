import { useContext } from 'react'
import { ChatContext } from '@/contexts/ChatContext'

const useChatContext = () => {
  const ctx = useContext(ChatContext)
  if (!ctx) throw new Error('useChatContext must be used within ChatProvider')
  return ctx
}

export { useChatContext }

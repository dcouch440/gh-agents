import { useMemo } from 'react'

const THINKING_RE = /<thinking>[\s\S]*?<\/thinking>/g

const useStripThinking = (content: string): string =>
  useMemo(() => content.replace(THINKING_RE, ''), [content])

export { useStripThinking }

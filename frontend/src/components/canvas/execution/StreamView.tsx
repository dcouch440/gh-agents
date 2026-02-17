import { useRef, useEffect } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type StreamViewProps = {
  content: string
  status: 'idle' | 'running' | 'completed' | 'failed'
  error?: string | null
  maxHeight?: number
}

function StreamView({ content, status, error, maxHeight }: StreamViewProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const shouldAutoScrollRef = useRef(true)

  useEffect(() => {
    const el = containerRef.current
    if (el === null) return

    const handleScroll = () => {
      shouldAutoScrollRef.current =
        el.scrollTop + el.clientHeight >= el.scrollHeight - 20
    }

    el.addEventListener('scroll', handleScroll)
    return () => el.removeEventListener('scroll', handleScroll)
  }, [])

  useEffect(() => {
    const el = containerRef.current
    if (el === null || !shouldAutoScrollRef.current) return
    el.scrollTop = el.scrollHeight
  }, [content])

  if (content === '' && status === 'idle') return null

  return (
    <Box
      ref={containerRef}
      className="nowheel nodrag nopan"
      sx={{
        flex: 1,
        overflowY: 'auto',
        maxHeight,
      }}
    >
      <Box
        component="pre"
        sx={{
          m: 0,
          fontFamily: 'monospace',
          fontSize: 11,
          wordWrap: 'break-word',
          whiteSpace: 'pre-wrap',
          color: 'text.primary',
        }}
      >
        {content}
        {status === 'running' && (
          <Box
            component="span"
            sx={{
              '@keyframes blink': {
                '0%': { opacity: 0 },
                '100%': { opacity: 1 },
              },
              animation: 'blink 0.6s step-end infinite',
            }}
          >
            {'\u258C'}
          </Box>
        )}
      </Box>

      {error !== undefined && error !== null && (
        <Box sx={{ backgroundColor: '#f8514920', px: 1.5, py: 0.5 }}>
          <Typography sx={{ color: '#f85149', fontSize: 10 }}>
            {error}
          </Typography>
        </Box>
      )}
    </Box>
  )
}

export { StreamView }
export type { StreamViewProps }

import { useState } from 'react'
import Box from '@mui/material/Box'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { CONTEXT_NODE } from './constants'

type ContentViewMode = 'raw' | 'md'

type ContextNodeContentProps = {
  content: string
  accentColor?: string
  onChange: (value: string) => void
}

function ContextNodeContent({ content, accentColor = CONTEXT_NODE.ACCENT_COLOR, onChange }: ContextNodeContentProps) {
  const [viewMode, setViewMode] = useState<ContentViewMode>('raw')

  return (
    <Box sx={{ position: 'relative', height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Raw / Md toggle */}
      <Box
        sx={{
          position: 'absolute',
          top: 6,
          right: 24,
          display: 'flex',
          gap: 0.25,
          zIndex: 1,
        }}
      >
        {(['raw', 'md'] as const).map((vm) => (
          <Box
            key={vm}
            onClick={() => {
              setViewMode(vm)
            }}
            sx={{
              px: 0.75,
              py: 0.25,
              borderRadius: '4px',
              fontSize: 10,
              fontWeight: 600,
              cursor: 'pointer',
              userSelect: 'none',
              color: viewMode === vm ? accentColor : 'text.disabled',
              backgroundColor: viewMode === vm ? `${accentColor}15` : 'transparent',
              transition: 'all 120ms ease',
              '&:hover': viewMode === vm ? {} : { color: 'text.secondary' },
            }}
          >
            {vm === 'raw' ? 'Raw' : 'Md'}
          </Box>
        ))}
      </Box>

      {/* Content area */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, overflow: 'hidden', pt: 0.5, px: 0.5, pb: 0.5 }}>
        {viewMode === 'raw' ? (
          <CodeEditor
            value={content}
            onChange={onChange}
            placeholder="Type your context input here..."
            height="100%"
          />
        ) : (
          <Box sx={{ px: 1, py: 0.5, overflow: 'auto', height: '100%' }}>
            <MarkdownPreview content={content} />
          </Box>
        )}
      </Box>
    </Box>
  )
}

export { ContextNodeContent }
export type { ContextNodeContentProps }

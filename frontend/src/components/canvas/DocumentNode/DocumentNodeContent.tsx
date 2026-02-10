import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { DOCUMENT_NODE } from './constants'
import type { DocumentNodeMode } from './types'

type ContentViewMode = 'raw' | 'md'

type DocumentNodeContentProps = {
  content: string
  mode: DocumentNodeMode
  accentColor?: string
  onChange: (value: string) => void
}

function DocumentNodeContent({
  content,
  mode,
  accentColor = DOCUMENT_NODE.ACCENT_COLOR,
  onChange,
}: DocumentNodeContentProps) {
  const [viewMode, setViewMode] = useState<ContentViewMode>(mode === 'entry' ? 'raw' : 'md')

  const isEditable = mode === 'entry'
  const isEmpty = !content.trim()

  return (
    <Box sx={{ position: 'relative', height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Raw / Md toggle — top-right corner */}
      <Box
        sx={{
          position: 'absolute',
          top: 6,
          right: 8,
          display: 'flex',
          gap: 0.25,
          zIndex: 1,
        }}
      >
        {(['raw', 'md'] as const).map((vm) => (
          <Box
            key={vm}
            onClick={() => { setViewMode(vm) }}
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
      <Box
        className="nowheel nodrag nopan"
        sx={{ flex: 1, overflow: 'hidden', pt: 0.5, px: 0.5, pb: 0.5 }}
      >
        {viewMode === 'raw' ? (
          <CodeEditor
            value={content}
            onChange={onChange}
            readOnly={!isEditable}
            placeholder={isEditable ? 'Type your test input here...' : undefined}
            height="100%"
          />
        ) : isEmpty && mode === 'document' ? (
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
            }}
          >
            <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
              Document will be generated when workflow runs.
            </Typography>
          </Box>
        ) : (
          <Box sx={{ px: 1, py: 0.5, overflow: 'auto', height: '100%' }}>
            <MarkdownPreview content={content} />
          </Box>
        )}
      </Box>
    </Box>
  )
}

export { DocumentNodeContent }
export type { DocumentNodeContentProps }

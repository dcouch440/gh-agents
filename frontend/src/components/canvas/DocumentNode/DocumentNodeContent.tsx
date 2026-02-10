import { useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'
import { CodeEditor } from '@/components/primitives/CodeEditor'
import { DOCUMENT_NODE } from './constants'

const noop = () => {}

type ContentViewMode = 'raw' | 'md'

type DocumentNodeContentProps = {
  content: string
  accentColor?: string
}

function DocumentNodeContent({ content, accentColor = DOCUMENT_NODE.ACCENT_COLOR }: DocumentNodeContentProps) {
  const [viewMode, setViewMode] = useState<ContentViewMode>('md')

  const isEmpty = !content.trim()

  return (
    <Box sx={{ position: 'relative', height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Raw / Md toggle — top-right corner */}
      {!isEmpty && (
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
      )}

      {/* Content area */}
      <Box className="nowheel nodrag nopan" sx={{ flex: 1, overflow: 'hidden', pt: 0.5, px: 0.5, pb: 0.5 }}>
        {isEmpty ? (
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
        ) : viewMode === 'raw' ? (
          <CodeEditor value={content} onChange={noop} readOnly height="100%" />
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

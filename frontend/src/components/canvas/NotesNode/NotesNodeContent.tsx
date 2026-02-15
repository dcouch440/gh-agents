import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { MarkdownPreview } from '@/components/primitives/MarkdownPreview'

type NotesNodeContentProps = {
  content: string
}

function NotesNodeContent({ content }: NotesNodeContentProps) {
  const isEmpty = !content.trim()

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
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
              Notes will appear as the assistant records them.
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

export { NotesNodeContent }
export type { NotesNodeContentProps }

import Box from '@mui/material/Box'
import { CodeEditor } from '@/components/primitives/CodeEditor'

type PromptTabProps = {
  value: string
  onChange: (value: string) => void
}

function PromptTab({ value, onChange }: PromptTabProps) {
  return (
    <Box
      className="nowheel nodrag nopan"
      sx={{
        width: '100%',
        height: '100%',
        '& > div': {
          height: '100%',
          border: 'none',
          borderRadius: 0,
        },
      }}
    >
      <CodeEditor value={value} onChange={onChange} placeholder="Enter your prompt..." height="100%" />
    </Box>
  )
}

export { PromptTab }
export type { PromptTabProps }

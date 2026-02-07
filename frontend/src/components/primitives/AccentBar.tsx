import Box from '@mui/material/Box'

type AccentBarProps = {
  color: string
}

function AccentBar({ color }: AccentBarProps) {
  return (
    <Box
      sx={{
        width: 3,
        alignSelf: 'stretch',
        flexShrink: 0,
        borderRadius: '0 2px 2px 0',
        backgroundColor: color,
      }}
    />
  )
}

export { AccentBar }
export type { AccentBarProps }

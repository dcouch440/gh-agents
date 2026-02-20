import Box from '@mui/material/Box'

function ThinkingIndicator() {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, py: 1, px: 0.25 }}>
      {[0, 1, 2].map((i) => (
        <Box
          key={i}
          sx={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            bgcolor: 'text.secondary',
            opacity: 0.4,
            animation: 'thinkingBounce 1.4s ease-in-out infinite',
            animationDelay: `${i * 0.16}s`,
            '@keyframes thinkingBounce': {
              '0%, 80%, 100%': { opacity: 0.4, transform: 'scale(1)' },
              '40%': { opacity: 1, transform: 'scale(1.2)' },
            },
          }}
        />
      ))}
    </Box>
  )
}

export { ThinkingIndicator }

import Box from '@mui/material/Box'

const WIDTHS = ['70%', '45%', '60%']

function SkeletonLines() {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: '12px', py: 1 }}>
      {WIDTHS.map((w, i) => (
        <Box
          key={i}
          sx={{
            height: 2,
            width: w,
            borderRadius: 1,
            backgroundColor: 'text.disabled',
            animation: 'skeletonPulse 2s ease-in-out infinite',
            animationDelay: `${i * 0.3}s`,
            '@keyframes skeletonPulse': {
              '0%, 100%': { opacity: 0.3 },
              '50%': { opacity: 0.7 },
            },
          }}
        />
      ))}
    </Box>
  )
}

export { SkeletonLines }

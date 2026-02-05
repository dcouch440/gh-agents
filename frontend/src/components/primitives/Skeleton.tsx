import { Skeleton as MuiSkeleton, Box } from '@mui/material'

type SkeletonVariant = 'card' | 'table-row' | 'text-block'

type SkeletonProps = {
  variant?: SkeletonVariant
  count?: number
}

const renderCard = () => (
  <Box sx={{ p: 2, border: 1, borderColor: 'divider', borderRadius: 2 }}>
    <MuiSkeleton variant="text" width="60%" height={28} />
    <MuiSkeleton variant="text" width="80%" height={20} sx={{ mt: 1 }} />
    <MuiSkeleton variant="text" width="40%" height={20} sx={{ mt: 0.5 }} />
  </Box>
)

const renderTableRow = () => (
  <Box sx={{ display: 'flex', gap: 2, py: 1.5, borderBottom: 1, borderColor: 'divider' }}>
    <MuiSkeleton variant="text" width="30%" height={20} />
    <MuiSkeleton variant="text" width="25%" height={20} />
    <MuiSkeleton variant="text" width="15%" height={20} />
    <MuiSkeleton variant="text" width="20%" height={20} />
  </Box>
)

const renderTextBlock = () => (
  <Box>
    <MuiSkeleton variant="text" width="90%" height={20} />
    <MuiSkeleton variant="text" width="75%" height={20} />
    <MuiSkeleton variant="text" width="60%" height={20} />
  </Box>
)

const VARIANT_RENDERERS = {
  card: renderCard,
  'table-row': renderTableRow,
  'text-block': renderTextBlock,
} as const

function Skeleton({ variant = 'text-block', count = 1 }: SkeletonProps) {
  const render = VARIANT_RENDERERS[variant]

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
      {Array.from({ length: count }, (_, i) => (
        <Box key={i}>{render()}</Box>
      ))}
    </Box>
  )
}

export { Skeleton }
export type { SkeletonProps, SkeletonVariant }

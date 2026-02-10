import { Box, Typography } from '@mui/material'
import type { UsageSummary } from '@/types'
import { Collections } from '@/utils/collections'

type TokenUsageStatusProps = {
  usage: UsageSummary[]
}

const fmtTokens = (n: number): string => {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return `${n}`
}

function TokenUsageStatus({ usage }: TokenUsageStatusProps) {
  const totals = Collections.aggregate(usage, {
    input: (r) => r.total_input,
    output: (r) => r.total_output,
    calls: (r) => r.call_count,
  })

  return (
    <Box sx={{ fontSize: '0.75rem', lineHeight: 1.4 }}>
      <Box
        sx={{
          display: 'flex',
          gap: 2,
          py: '1px',
          color: 'text.disabled',
          borderBottom: 1,
          borderColor: 'divider',
        }}
      >
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            flex: 1,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          MODEL
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          CALLS
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          IN
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          OUT
        </Typography>
      </Box>

      {usage.map((row) => (
        <Box
          key={row.model_id}
          sx={{
            display: 'flex',
            gap: 2,
            py: '1px',
            borderBottom: '1px solid',
            borderColor: 'divider',
          }}
        >
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              flex: 1,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {row.model_id}
          </Typography>
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              width: '6ch',
              flexShrink: 0,
              textAlign: 'right',
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {row.call_count}
          </Typography>
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              width: '6ch',
              flexShrink: 0,
              textAlign: 'right',
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {fmtTokens(row.total_input)}
          </Typography>
          <Typography
            component="span"
            sx={{
              fontSize: 'inherit',
              lineHeight: 'inherit',
              width: '6ch',
              flexShrink: 0,
              textAlign: 'right',
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {fmtTokens(row.total_output)}
          </Typography>
        </Box>
      ))}

      <Box
        sx={{
          display: 'flex',
          gap: 2,
          py: '1px',
          borderTop: 1,
          borderColor: 'divider',
          color: 'text.primary',
        }}
      >
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            flex: 1,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          TOTAL
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {totals.calls}
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {fmtTokens(totals.input)}
        </Typography>
        <Typography
          component="span"
          sx={{
            fontSize: 'inherit',
            lineHeight: 'inherit',
            width: '6ch',
            flexShrink: 0,
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {fmtTokens(totals.output)}
        </Typography>
      </Box>
    </Box>
  )
}

export { TokenUsageStatus }
export type { TokenUsageStatusProps }

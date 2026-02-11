import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { EmptyState } from '@/components/primitives'
import { DEFAULT_STEP_TYPE_COLOR, PROTOCOL_TYPE_COLORS } from '@/components/canvas/constants'
import { SECTION_LABEL_SX } from './constants'
import type { Protocol } from '@/types/protocol'
import type { Agent } from '@/types/agent'

type ProtocolTabProps = {
  allProtocols: Protocol[]
  agentsById: Map<string, Agent>
}

function ProtocolTab({ allProtocols, agentsById }: ProtocolTabProps) {
  if (allProtocols.length === 0) {
    return (
      <Box sx={{ flex: 1, overflow: 'auto' }}>
        <Typography sx={SECTION_LABEL_SX}>Available Protocols</Typography>
        <EmptyState message="No protocols available" />
      </Box>
    )
  }

  return (
    <Box sx={{ flex: 1, overflow: 'auto' }}>
      <Typography sx={SECTION_LABEL_SX}>Available Protocols</Typography>
      {allProtocols.map((proto) => {
        const protoColor = PROTOCOL_TYPE_COLORS[proto.protocol_type] ?? DEFAULT_STEP_TYPE_COLOR
        return (
          <Box
            key={proto.id}
            sx={{
              borderBottom: 1,
              borderColor: 'divider',
              px: '16px',
              py: '10px',
            }}
          >
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
              <Box
                sx={{
                  px: '6px',
                  py: '2px',
                  borderRadius: '4px',
                  backgroundColor: `${protoColor}20`,
                }}
              >
                <Typography
                  sx={{
                    fontSize: 9,
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    color: protoColor,
                    letterSpacing: '0.06em',
                    lineHeight: 1,
                  }}
                >
                  {proto.protocol_type}
                </Typography>
              </Box>
              <Typography
                sx={{
                  fontSize: 12,
                  fontWeight: 600,
                  color: 'text.primary',
                  flex: 1,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {proto.name}
              </Typography>
            </Box>
            <Typography
              sx={{
                fontSize: 11,
                color: 'text.secondary',
                lineHeight: 1.4,
                mb: proto.ports.length > 0 ? 1 : 0,
              }}
            >
              {proto.description}
            </Typography>
            {proto.ports.length > 0 && (
              <Box>
                <Typography
                  sx={{
                    fontSize: 9,
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    color: 'text.disabled',
                    letterSpacing: '0.06em',
                    mb: 0.5,
                  }}
                >
                  Ports ({proto.ports.length})
                </Typography>
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                  {proto.ports.map((port) => {
                    const portAgent = port.agent_id ? agentsById.get(port.agent_id) : undefined
                    return (
                      <Box
                        key={port.id}
                        sx={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: 0.5,
                          px: '6px',
                          py: '2px',
                          borderRadius: '4px',
                          backgroundColor: `${protoColor}10`,
                          border: 1,
                          borderColor: `${protoColor}25`,
                        }}
                      >
                        <Typography sx={{ fontSize: 10, color: 'text.secondary', fontWeight: 500 }}>{port.port_name}</Typography>
                        {portAgent ? <Typography sx={{ fontSize: 9, color: 'text.disabled' }}>{portAgent.name}</Typography> : null}
                      </Box>
                    )
                  })}
                </Box>
              </Box>
            )}
            {proto.agent ? (
              <Box sx={{ mt: 0.75 }}>
                <Typography sx={{ fontSize: 10, color: 'text.disabled' }}>
                  Agent: {proto.agent.name} ({proto.agent.model_id})
                </Typography>
              </Box>
            ) : null}
          </Box>
        )
      })}
    </Box>
  )
}

export { ProtocolTab }
export type { ProtocolTabProps }

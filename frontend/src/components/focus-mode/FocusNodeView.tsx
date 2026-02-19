import Box from '@mui/material/Box'
import { useTheme } from '@mui/material/styles'
import { useStore, workflowStore } from '@/stores'
import { FOCUS_MODE } from '@/constants'
import { ARCHETYPE_CONFIGS } from '@/components/canvas/DynamicNode/archetypes'
import type { Archetype as ArchetypeType } from '@/components/canvas/DynamicNode/archetypes'
import { useStepStoreData } from '@/components/canvas/DynamicNode/useStepStoreData'
import { buildStepTabs } from '@/components/canvas/DynamicNode/buildStepTabs'
import { resolveSubtitle } from '@/components/canvas/DynamicNode/resolveSubtitle'
import { FocusHeader } from './FocusHeader'
import { FocusTabStrip } from './FocusTabStrip'

type FocusNodeViewProps = {
  stepId: string
  archetype: ArchetypeType
  stepName: string
  activeTabId: string
  onTabChange: (tabId: string) => void
}

function FocusNodeView({
  stepId,
  archetype,
  stepName,
  activeTabId,
  onTabChange,
}: FocusNodeViewProps) {
  const theme = useTheme()
  const config = ARCHETYPE_CONFIGS[archetype]
  const accentColor = config.color

  const { roomStepMembers, stepIssues } = useStepStoreData(stepId)
  const rosterAgents = useStore(workflowStore.store, workflowStore.selectStepRoster(stepId))

  const tabs = buildStepTabs({
    stepId,
    archetype,
    focusMode: true,
  })

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0]

  const subtitle = resolveSubtitle({
    archetype,
    rosterNames: rosterAgents.map((a) => a.name),
    roomMemberNames: roomStepMembers.map((m) => m.name),
  })

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        backgroundColor: theme.palette.background.default,
      }}
    >
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
          maxWidth: FOCUS_MODE.CONTENT_MAX_WIDTH,
          width: '100%',
          mx: 'auto',
        }}
      >
        <FocusHeader
          name={stepName}
          archetype={archetype}
          subtitle={subtitle}
          issueCount={stepIssues.length}
          issueDescriptions={stepIssues.map((i) => i.description)}
        />

        <FocusTabStrip
          tabs={tabs}
          activeTabId={activeTabId}
          onTabChange={onTabChange}
          accentColor={accentColor}
        />

        <Box
          sx={{
            flex: 1,
            minHeight: 0,
            overflow: 'hidden',
            position: 'relative',
            cursor: 'text',
            userSelect: 'text',
          }}
        >
          {activeTab?.content}
        </Box>
      </Box>
    </Box>
  )
}

export { FocusNodeView }
export type { FocusNodeViewProps }

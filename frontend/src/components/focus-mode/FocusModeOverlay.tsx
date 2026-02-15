import { useMemo, useCallback } from 'react'
import { createPortal } from 'react-dom'
import Box from '@mui/material/Box'
import IconButton from '@mui/material/IconButton'
import CloseOutlined from '@mui/icons-material/CloseOutlined'
import { Tooltip } from '@/components/primitives/Tooltip'
import { useTheme } from '@mui/material/styles'
import { motion, AnimatePresence } from 'framer-motion'
import { useStore, workflowStore, canvasStore, focusModeStore } from '@/stores'
import { FOCUS_MODE } from '@/constants'
import { useReducedMotion } from '@/hooks/useReducedMotion'
import { useFocusNavigation } from '@/hooks/useFocusNavigation'
import { resolveArchetype, ARCHETYPE_CONFIGS } from '@/components/canvas/DynamicNode/archetypes'
import { STEP_TYPE_COLORS, DEFAULT_STEP_TYPE_COLOR } from '@/components/canvas/constants'
import { Collections } from '@/utils/collections'
import { ArtifactBar } from './ArtifactBar'
import type { StepSection, CardEntry } from './ArtifactBar'
import { FocusNodeView } from './FocusNodeView'
import { FocusNavBar } from './FocusNavBar'
import { ArtifactDetailPanel } from './ArtifactDetailPanel'

// ── Slide variants ───────────────────────────────────────────────────────────

const slideVariants = {
  enter: (direction: 'left' | 'right' | 'none') => ({
    x: direction === 'left' ? '100%' : direction === 'right' ? '-100%' : 0,
    opacity: 0,
  }),
  center: { x: 0, opacity: 1 },
  exit: (direction: 'left' | 'right' | 'none') => ({
    x: direction === 'left' ? '-100%' : direction === 'right' ? '100%' : 0,
    opacity: 0,
  }),
}

// ── Component ────────────────────────────────────────────────────────────────

function FocusModeOverlay() {
  const theme = useTheme()
  const reducedMotion = useReducedMotion()
  const { touchHandlers } = useFocusNavigation()

  const active = useStore(focusModeStore.store, focusModeStore.selectActive)
  const currentStepId = useStore(focusModeStore.store, focusModeStore.selectCurrentStepId)
  const currentIndex = useStore(focusModeStore.store, focusModeStore.selectCurrentIndex)
  const stepCount = useStore(focusModeStore.store, focusModeStore.selectStepCount)
  const orderedStepIds = useStore(focusModeStore.store, focusModeStore.selectOrderedStepIds)
  const slideDirection = useStore(focusModeStore.store, focusModeStore.selectSlideDirection)
  const activeTabId = useStore(focusModeStore.store, focusModeStore.selectActiveTabId)
  const expandedArtifactId = useStore(focusModeStore.store, focusModeStore.selectExpandedArtifactId)
  const expandedArtifactKind = useStore(focusModeStore.store, focusModeStore.selectExpandedArtifactKind)

  const currentStep = useStore(workflowStore.store, workflowStore.selectStepById(currentStepId))
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const roomMembersByStep = useStore(workflowStore.store, workflowStore.selectRoomMembersByStep)
  const documentDefsByStep = useStore(workflowStore.store, workflowStore.selectDocumentDefsByStep)
  const protocolsByStep = useStore(canvasStore.store, canvasStore.selectStepProtocols)

  // Build a Map<string, ProtocolStepInfo> from the canvas store's Record format
  const protocolsMap = useMemo(() => {
    const map = new Map<string, { protocol_type: string; name: string; portNames: string[] }>()
    for (const [stepId, link] of Object.entries(protocolsByStep)) {
      map.set(stepId, {
        protocol_type: link.protocolType,
        name: link.protocolName,
        portNames: link.portNames,
      })
    }
    return map
  }, [protocolsByStep])

  // Resolve archetype for current step
  const currentArchetype = useMemo(() => {
    if (!currentStep) return null
    return resolveArchetype(currentStep, protocolsMap, currentStep.id)
  }, [currentStep, protocolsMap])

  // Compute step names map for nav bar and upstream resolution
  const stepNamesMap = useMemo(
    () => Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? 'Unnamed'),
    [steps],
  )

  // Compute upstream step names for the current step
  const upstreamStepNames = useMemo(() => {
    if (!currentStepId) return []
    const upstream: string[] = []
    for (let i = 0; i < edges.length; i++) {
      const e = edges[i]!
      if (e.to_step_id === currentStepId) {
        upstream.push(stepNamesMap.get(e.from_step_id) ?? 'Unknown Step')
      }
    }
    return upstream
  }, [currentStepId, edges, stepNamesMap])

  // Compute accent colors for nav dots (use STEP_TYPE_COLORS for input/context)
  const accentColors = useMemo(() => {
    const colors: string[] = []
    for (let i = 0; i < orderedStepIds.length; i++) {
      const id = orderedStepIds[i]!
      const step = steps.find((s) => s.id === id)
      if (step) {
        if (step.execution_mode === 'input' || step.execution_mode === 'context') {
          colors.push(STEP_TYPE_COLORS[step.execution_mode] ?? DEFAULT_STEP_TYPE_COLOR)
        } else {
          const arch = resolveArchetype(step, protocolsMap, id)
          colors.push(ARCHETYPE_CONFIGS[arch].color)
        }
      } else {
        colors.push('#7d8590')
      }
    }
    return colors
  }, [orderedStepIds, steps, protocolsMap])

  // Build per-step sections for the artifact bar
  const stepSections = useMemo((): readonly StepSection[] => {
    const sections: StepSection[] = []
    for (let i = 0; i < orderedStepIds.length; i++) {
      const id = orderedStepIds[i]!
      const step = steps.find((s) => s.id === id)
      if (!step) continue

      const stepName = step.name ?? 'Unnamed'
      const color = accentColors[i] ?? '#7d8590'
      const cards: CardEntry[] = []

      switch (step.execution_mode) {
        case 'task_force': {
          const roster = rosterByStep[id] ?? []
          if (roster.length > 0) {
            for (let j = 0; j < roster.length; j++) {
              const agent = roster[j]!
              cards.push({ id: agent.id, name: agent.name, subtitle: agent.role_description })
            }
          } else {
            cards.push({ id, name: stepName, subtitle: 'No agents' })
          }
          break
        }
        case 'documenter': {
          const docs = documentDefsByStep[id] ?? []
          if (docs.length > 0) {
            for (let j = 0; j < docs.length; j++) {
              const d = docs[j]!
              cards.push({ id: d.id, name: d.name, subtitle: `~${d.target_length} chars` })
            }
          } else {
            cards.push({ id, name: stepName, subtitle: 'No documents' })
          }
          break
        }
        case 'room': {
          const members = roomMembersByStep[id] ?? []
          if (members.length > 0) {
            for (let j = 0; j < members.length; j++) {
              const m = members[j]!
              cards.push({ id: m.id, name: m.name, subtitle: m.role })
            }
          } else {
            cards.push({ id, name: stepName, subtitle: 'No members' })
          }
          break
        }
        default:
          cards.push({ id, name: stepName, subtitle: null })
          break
      }

      sections.push({ stepId: id, stepName, accentColor: color, cards })
    }
    return sections
  }, [orderedStepIds, steps, accentColors, rosterByStep, documentDefsByStep, roomMembersByStep])

  const handleStepCardClick = useCallback((stepId: string) => {
    const idx = orderedStepIds.indexOf(stepId)
    if (idx >= 0) {
      focusModeStore.goToIndex(idx)
    }
  }, [orderedStepIds])

  const handleCollapseArtifact = useCallback(() => {
    focusModeStore.collapseArtifact()
  }, [])

  const handleTabChange = useCallback((tabId: string) => {
    focusModeStore.setActiveTab(tabId)
  }, [])

  if (!active) return null

  const currentStepName = currentStep?.name ?? 'Unnamed'

  const overlay = (
    <Box
      {...touchHandlers}
      sx={{
        position: 'fixed',
        inset: 0,
        zIndex: FOCUS_MODE.Z_INDEX,
        display: 'flex',
        flexDirection: 'column',
        backgroundColor: theme.palette.background.default,
        outline: 'none',
      }}
    >
      {/* Top bar: artifacts + close button */}
      <Box sx={{ position: 'relative', flexShrink: 0 }}>
        <ArtifactBar
          sections={stepSections}
          currentStepId={currentStepId}
          onStepClick={handleStepCardClick}
        />
        <Tooltip title="Exit Focus Mode (Esc)" placement="bottom">
          <IconButton
            onClick={focusModeStore.exit}
            size="small"
            sx={{
              position: 'absolute',
              top: 8,
              right: 8,
              width: 32,
              height: 32,
              zIndex: 1,
              color: 'text.secondary',
              backgroundColor: theme.palette.background.default,
              border: 1,
              borderColor: 'divider',
              '&:hover': { color: 'text.primary', backgroundColor: theme.palette.background.paper },
            }}
          >
            <CloseOutlined sx={{ fontSize: 18 }} />
          </IconButton>
        </Tooltip>
      </Box>

      {/* Node view with slide transitions */}
      <Box sx={{ flex: 1, minHeight: 0, position: 'relative', overflow: 'hidden' }}>
        <AnimatePresence mode="popLayout" custom={slideDirection}>
          {currentStepId !== null && currentArchetype !== null && (
            <motion.div
              key={currentStepId}
              custom={slideDirection}
              variants={slideVariants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={{ duration: reducedMotion ? 0 : 0.25, ease: 'easeInOut' }}
              style={{ position: 'absolute', inset: 0 }}
            >
              <FocusNodeView
                stepId={currentStepId}
                archetype={currentArchetype}
                stepName={currentStepName}
                upstreamStepNames={upstreamStepNames}
                activeTabId={activeTabId}
                onTabChange={handleTabChange}
              />
            </motion.div>
          )}
        </AnimatePresence>

        {/* Artifact detail panel (overlays node view) */}
        {expandedArtifactId !== null && expandedArtifactKind !== null && (
          <ArtifactDetailPanel
            artifactId={expandedArtifactId}
            artifactKind={expandedArtifactKind}
            onClose={handleCollapseArtifact}
          />
        )}
      </Box>

      {/* Navigation bar */}
      <FocusNavBar
        stepCount={stepCount}
        currentIndex={currentIndex}
        currentStepName={currentStepName}
        accentColors={accentColors}
        onPrev={focusModeStore.goPrev}
        onNext={focusModeStore.goNext}
        onDotClick={focusModeStore.goToIndex}
      />
    </Box>
  )

  return createPortal(overlay, document.body)
}

export { FocusModeOverlay }

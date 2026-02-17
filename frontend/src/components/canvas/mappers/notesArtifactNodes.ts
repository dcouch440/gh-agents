import type { Node } from '@xyflow/react'
import type { WorkflowStep } from '@/types/workflow'
import { NOTES_NODE } from '../NotesNode'
import type { NotesNodeData } from '../NotesNode'
import { CanvasNodeKind } from '../canvasKinds'
import { getStoredDimensions, getStoredPosition } from '../nodeResizeStorage'
import type { StepNodeLookups } from './types'

const toNotesArtifactNodes = (
  steps: WorkflowStep[],
  lookups: StepNodeLookups,
): Node[] => {
  const notesNodes: Node[] = []

  for (const step of steps) {
    if (step.execution_mode === 'context' || step.execution_mode === 'input') continue
    const content = lookups.notesByStep[step.id]
    if (!content) continue

    const notesData: NotesNodeData = {
      kind: CanvasNodeKind.NOTES,
      label: 'Agent Notes',
      stepName: step.name ?? step.execution_mode,
      content,
      protocolStepId: step.id,
    }
    const notesNodeId = `notes-${step.id}`
    const notesDims = getStoredDimensions(notesNodeId)
    const notesPos = getStoredPosition(notesNodeId)
    notesNodes.push({
      id: notesNodeId,
      type: 'notesNode',
      position: notesPos ?? {
        x: (step.position_x ?? 0),
        y: (step.position_y ?? 0) + NOTES_NODE.DEFAULT_HEIGHT + 40,
      },
      style: {
        width: notesDims?.width ?? NOTES_NODE.DEFAULT_WIDTH,
        height: notesDims?.height ?? NOTES_NODE.DEFAULT_HEIGHT,
      },
      draggable: true,
      connectable: false,
      data: notesData,
    })
  }

  return notesNodes
}

export { toNotesArtifactNodes }

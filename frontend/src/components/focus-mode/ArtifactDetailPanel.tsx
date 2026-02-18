import type { ArtifactKind } from '@/stores'
import { StepContentDetail } from './StepContentDetail'
import { AgentDetail } from './AgentDetail'
import { MemberDetail } from './MemberDetail'
import { TaskForceDetail } from './TaskForceDetail'
import { RoomDetail } from './RoomDetail'

type ArtifactDetailPanelProps = {
  artifactId: string
  artifactKind: ArtifactKind
  onClose: () => void
}

function ArtifactDetailPanel({ artifactId, artifactKind, onClose }: ArtifactDetailPanelProps) {
  if (artifactKind === 'input' || artifactKind === 'context') {
    return <StepContentDetail stepId={artifactId} kind={artifactKind} onClose={onClose} />
  }
  if (artifactKind === 'roster-agent') {
    return <AgentDetail artifactId={artifactId} onClose={onClose} />
  }
  if (artifactKind === 'room-member') {
    return <MemberDetail artifactId={artifactId} onClose={onClose} />
  }
  if (artifactKind === 'task-force') {
    return <TaskForceDetail stepId={artifactId} onClose={onClose} />
  }
  return <RoomDetail stepId={artifactId} onClose={onClose} />
}

export { ArtifactDetailPanel }
export type { ArtifactDetailPanelProps }

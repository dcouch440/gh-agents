import { useStore, workflowStore } from '@/stores'
import type { RoomStepMember } from '@/types/workflow'
import type { ConsistencyIssue } from '@/types/ws'

type StepStoreData = {
  roomStepMembers: RoomStepMember[]
  stepIssues: ConsistencyIssue[]
}

const useStepStoreData = (stepId: string): StepStoreData => {
  const roomStepMembers = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(stepId))
  const stepIssues = useStore(workflowStore.store, workflowStore.selectStepIssues(stepId))

  return { roomStepMembers, stepIssues }
}

export { useStepStoreData }
export type { StepStoreData }

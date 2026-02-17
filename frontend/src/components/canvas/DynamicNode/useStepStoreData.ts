import { useStore, workflowStore } from '@/stores'
import type { DocumentDef, RoomStepMember } from '@/types/workflow'
import type { ConsistencyIssue } from '@/types/ws'

type StepStoreData = {
  documentDefs: DocumentDef[]
  roomStepMembers: RoomStepMember[]
  stepIssues: ConsistencyIssue[]
}

const useStepStoreData = (stepId: string): StepStoreData => {
  const documentDefs = useStore(workflowStore.store, workflowStore.selectStepDocumentDefs(stepId))
  const roomStepMembers = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(stepId))
  const stepIssues = useStore(workflowStore.store, workflowStore.selectStepIssues(stepId))

  return { documentDefs, roomStepMembers, stepIssues }
}

export { useStepStoreData }
export type { StepStoreData }

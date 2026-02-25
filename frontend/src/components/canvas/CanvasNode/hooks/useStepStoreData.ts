import { useStore, workflowStore } from '@/stores'
import type { RoomStepMember } from '@/types/workflow'

type StepStoreData = {
  roomStepMembers: RoomStepMember[]
}

const useStepStoreData = (stepId: string): StepStoreData => {
  const roomStepMembers = useStore(workflowStore.store, workflowStore.selectRoomStepMembers(stepId))

  return { roomStepMembers }
}

export { useStepStoreData }
export type { StepStoreData }

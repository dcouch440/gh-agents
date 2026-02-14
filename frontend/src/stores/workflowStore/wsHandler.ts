import { nmSet } from '../lib'
import { api } from '@/api'
import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage, DocDefChangedData, DocDefDeletedData, StepConfigUpdatedData, RosterChangedData, RoomMembersChangedData } from '@/types/ws'
import { store, getActiveId } from './_store'
import { fetchDocumentDefs } from './documents'
import { fetchRoster, fetchRoomStepMembers } from './roster'

/** Fetch a single step from the API and patch it into the store silently.
 *  Skips the update if the step has unsaved local edits (dirty). */
const refetchStep = async (workflowId: string, stepId: string): Promise<void> => {
  try {
    // Don't overwrite local dirty edits
    if (store.getState().dirtyStepIds.has(stepId)) return

    const step = await api.workflows.getStep(workflowId, stepId)
    store.setState((s) => {
      // Re-check after async gap
      if (s.dirtyStepIds.has(stepId)) return {}
      return { steps: nmSet(s.steps, stepId, step) }
    })
  } catch (err) {
    console.error(`[workflowStore] Failed to refetch step ${stepId}:`, err)
  }
}

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    const activeId = getActiveId()

    switch (msg.event) {
      case WORKFLOW_EVENT.DOC_DEF_CREATED:
      case WORKFLOW_EVENT.DOC_DEF_UPDATED:
      case WORKFLOW_EVENT.DOC_DEF_DELETED: {
        const d = msg.data as DocDefChangedData | DocDefDeletedData
        if (d.workflow_id !== activeId) break
        void fetchDocumentDefs(d.step_id)
        break
      }
      case WORKFLOW_EVENT.STEP_CONFIG_UPDATED: {
        const d = msg.data as StepConfigUpdatedData
        if (d.workflow_id !== activeId) break
        void refetchStep(d.workflow_id, d.step_id)
        break
      }
      case WORKFLOW_EVENT.ROSTER_CHANGED: {
        const d = msg.data as RosterChangedData
        if (d.workflow_id !== activeId) break
        void fetchRoster(d.step_id)
        break
      }
      case WORKFLOW_EVENT.ROOM_MEMBERS_CHANGED: {
        const d = msg.data as RoomMembersChangedData
        if (d.workflow_id !== activeId) break
        void fetchRoomStepMembers(d.step_id)
        break
      }
    }
  } catch (err) {
    console.error(`[workflowStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }

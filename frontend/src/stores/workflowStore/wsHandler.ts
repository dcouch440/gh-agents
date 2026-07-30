import { nmGet, nmSet } from '../lib'
import { api } from '@/api'
import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage, StepConfigUpdatedData, StepNameUpdatedData, RosterChangedData, RoomMembersChangedData, PlanUpdatedData, StepPinChangedData, StepCreatedData, StepDeletedData, EdgeCreatedData, EdgeDeletedData } from '@/types/ws'
import { store, getActiveId } from './_store'
import { fetchRoster, fetchRoomStepMembers } from './roster'
import { refreshStepsAndEdges, refreshBoardElements } from './workflows'

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
      case WORKFLOW_EVENT.STEP_CONFIG_UPDATED: {
        const d = msg.data as StepConfigUpdatedData
        if (d.workflow_id !== activeId) break
        void refetchStep(d.workflow_id, d.step_id)
        break
      }
      case WORKFLOW_EVENT.STEP_NAME_UPDATED: {
        const d = msg.data as StepNameUpdatedData
        if (d.workflow_id !== activeId) break
        store.setState((s) => {
          const existing = nmGet(s.steps, d.step_id)
          if (!existing) return {}
          if (s.dirtyStepIds.has(d.step_id)) return {}
          return { steps: nmSet(s.steps, d.step_id, { ...existing, name: d.name }) }
        })
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
      case WORKFLOW_EVENT.PLAN_UPDATED: {
        const d = msg.data as PlanUpdatedData
        if (d.workflow_id !== activeId) break
        store.setState((s) => ({
          planByStep: { ...s.planByStep, [d.step_id]: d.content },
        }))
        break
      }
      case WORKFLOW_EVENT.STEP_PIN_CHANGED: {
        const d = msg.data as StepPinChangedData
        if (d.workflow_id !== activeId) break
        store.setState((s) => {
          const existing = nmGet(s.steps, d.step_id)
          if (!existing) return {}
          return { steps: nmSet(s.steps, d.step_id, { ...existing, pinned: d.pinned }) }
        })
        break
      }
      case WORKFLOW_EVENT.STEP_CREATED:
      case WORKFLOW_EVENT.STEP_DELETED:
      case WORKFLOW_EVENT.EDGE_CREATED:
      case WORKFLOW_EVENT.EDGE_DELETED: {
        const d = msg.data as StepCreatedData | StepDeletedData | EdgeCreatedData | EdgeDeletedData
        if (d.workflow_id !== activeId) break
        void refreshStepsAndEdges(d.workflow_id)
        break
      }
      case WORKFLOW_EVENT.BOARD_ELEMENTS_UPDATED: {
        const d = msg.data as { workflow_id: string }
        if (d.workflow_id !== activeId) break
        void refreshBoardElements(d.workflow_id)
        break
      }
    }
  } catch (err) {
    console.error(`[workflowStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }

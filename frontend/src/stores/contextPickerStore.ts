// ============================================================================
// contextPickerStore — Context picking mode for sending entity info to assistant
// ============================================================================

import { createStore, logger } from './lib'

// ── Types ────────────────────────────────────────────────────────────────────

type PickableEntityKind = 'agent' | 'prompt-template' | 'output-schema' | 'workflow-step' | 'document' | 'context-node'

type PickableEntity = {
  kind: PickableEntityKind
  id: string
  name: string
  summary: string
  data: Record<string, unknown>
}

type ContextPickerState = {
  active: boolean
  pendingEntity: PickableEntity | null
  targetStepId: string | null
}

// ── Constants ────────────────────────────────────────────────────────────────

const BODY_CLASS = 'nexor-context-picking'

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'contextPickerStore',
  createStore<ContextPickerState>(() => ({
    active: false,
    pendingEntity: null,
    targetStepId: null,
  })),
)

// ── Selectors ────────────────────────────────────────────────────────────────

const selectActive = (s: ContextPickerState): boolean => s.active

const selectPendingEntity = (s: ContextPickerState): PickableEntity | null => s.pendingEntity

const selectTargetStepId = (s: ContextPickerState): string | null => s.targetStepId

// ── Actions ──────────────────────────────────────────────────────────────────

const activate = (targetStepId: string): void => {
  store.setState({ active: true, targetStepId, pendingEntity: null })
  document.body.classList.add(BODY_CLASS)
}

const pick = (entity: PickableEntity): void => {
  store.setState({ pendingEntity: entity })
}

const deactivate = (): void => {
  store.setState({ active: false, pendingEntity: null, targetStepId: null })
  document.body.classList.remove(BODY_CLASS)
}

const dismissPending = (): void => {
  store.setState({ pendingEntity: null })
}

const reset = (): void => {
  deactivate()
}

// ── Export ────────────────────────────────────────────────────────────────────

export const contextPickerStore = {
  store,
  selectActive,
  selectPendingEntity,
  selectTargetStepId,
  activate,
  pick,
  deactivate,
  dismissPending,
  reset,
}

export type { ContextPickerState, PickableEntity, PickableEntityKind }

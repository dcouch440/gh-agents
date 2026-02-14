// ============================================================================
// contextMentionStore — Multi-select context mentions for chat input chips
// ============================================================================

import { createStore, logger } from './lib'

// ── Types ────────────────────────────────────────────────────────────────────

type PickableEntityKind = 'agent' | 'prompt-template' | 'output-schema' | 'workflow-step' | 'document' | 'context-node' | 'shared-field'

type PickableEntity = {
  kind: PickableEntityKind
  id: string
  name: string
  summary: string
  data: Record<string, unknown>
}

type MentionToken = {
  id: string
  entityId: string
  kind: PickableEntityKind
  label: string
  color: string
  entity: PickableEntity
  chipKey: string | null
  chipPreview: string | null
}

type AddMentionOptions = {
  chipKey?: string
  chipPreview?: string
}

type ContextMentionState = {
  /** Per-step mention lists. Key = stepId. */
  byStep: Readonly<Record<string, ReadonlyArray<MentionToken>>>
}

// ── Constants ────────────────────────────────────────────────────────────────

const EMPTY_MENTIONS: ReadonlyArray<MentionToken> = []

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'contextMentionStore',
  createStore<ContextMentionState>(() => ({
    byStep: {},
  })),
)

// ── Selectors ────────────────────────────────────────────────────────────────

const selectMentions =
  (stepId: string) =>
  (s: ContextMentionState): ReadonlyArray<MentionToken> =>
    s.byStep[stepId] ?? EMPTY_MENTIONS

const selectEntityIds =
  (stepId: string) =>
  (s: ContextMentionState): ReadonlySet<string> => {
    const mentions = s.byStep[stepId] ?? EMPTY_MENTIONS
    return new Set(mentions.map((m) => m.entityId))
  }

// ── Actions ──────────────────────────────────────────────────────────────────

const addMention = (stepId: string, entity: PickableEntity, color: string, options?: AddMentionOptions): void => {
  const current = store.getState().byStep[stepId] ?? []
  if (current.some((m) => m.entityId === entity.id)) return

  const token: MentionToken = {
    id: crypto.randomUUID(),
    entityId: entity.id,
    kind: entity.kind,
    label: entity.name,
    color,
    entity,
    chipKey: options?.chipKey ?? null,
    chipPreview: options?.chipPreview ?? null,
  }

  store.setState((s) => ({
    byStep: {
      ...s.byStep,
      [stepId]: [...(s.byStep[stepId] ?? []), token],
    },
  }))
}

const removeMention = (stepId: string, tokenId: string): void => {
  store.setState((s) => {
    const current = s.byStep[stepId]
    if (!current) return {}
    const next = current.filter((m) => m.id !== tokenId)
    return { byStep: { ...s.byStep, [stepId]: next } }
  })
}

const removeByEntityId = (stepId: string, entityId: string): void => {
  store.setState((s) => {
    const current = s.byStep[stepId]
    if (!current) return {}
    const next = current.filter((m) => m.entityId !== entityId)
    return { byStep: { ...s.byStep, [stepId]: next } }
  })
}

const clearStep = (stepId: string): void => {
  store.setState((s) => {
    const next = { ...s.byStep }
    delete next[stepId]
    return { byStep: next }
  })
}

const reset = (): void => {
  store.setState({ byStep: {} })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const contextMentionStore = {
  store,
  selectMentions,
  selectEntityIds,
  addMention,
  removeMention,
  removeByEntityId,
  clearStep,
  reset,
}

export type { ContextMentionState, MentionToken, PickableEntity, PickableEntityKind, AddMentionOptions }

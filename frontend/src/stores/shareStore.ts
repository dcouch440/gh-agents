// ============================================================================
// shareStore — Canvas-level share mode for injecting context into target chats
// ============================================================================

import { createStore, logger } from './lib'
import { Collections } from '@/utils/collections'
import { contextMentionStore } from './contextMentionStore'
import type { PickableEntity, PickableEntityKind } from './contextMentionStore'

// ── Types ────────────────────────────────────────────────────────────────────

type ShareableField = {
  key: string
  label: string
  category: string
  kind: PickableEntityKind
  entity: PickableEntity
  color: string
  chipKey: string
}

type ShareState = {
  active: boolean
  sourceStepId: string | null
  availableFields: ReadonlyArray<ShareableField>
  selectedKeys: ReadonlySet<string>
  pendingChatFocus: string | null
}

// ── Constants ────────────────────────────────────────────────────────────────

const INITIAL_STATE: ShareState = {
  active: false,
  sourceStepId: null,
  availableFields: [],
  selectedKeys: new Set(),
  pendingChatFocus: null,
}

const EMPTY_SET: ReadonlySet<string> = new Set()
const EMPTY_FIELDS: ReadonlyArray<ShareableField> = []

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'shareStore',
  createStore<ShareState>(() => ({ ...INITIAL_STATE })),
)

// ── Selectors ────────────────────────────────────────────────────────────────

const selectActive = (s: ShareState): boolean => s.active

const selectSourceStepId = (s: ShareState): string | null => s.sourceStepId

const selectAvailableFields = (s: ShareState): ReadonlyArray<ShareableField> =>
  s.active ? s.availableFields : EMPTY_FIELDS

const selectSelectedKeys = (s: ShareState): ReadonlySet<string> =>
  s.active ? s.selectedKeys : EMPTY_SET

const selectPendingChatFocus = (s: ShareState): string | null => s.pendingChatFocus

// ── Helpers ──────────────────────────────────────────────────────────────────

const CHIP_PREVIEW_MAX_LENGTH = 30

const truncatePreview = (value: string, maxLen: number = CHIP_PREVIEW_MAX_LENGTH): string =>
  value.length <= maxLen ? value : `${value.slice(0, maxLen)}\u2026`

const deriveChipPreview = (field: ShareableField): string => {
  if (field.kind === 'shared-field') {
    const value = typeof field.entity.data.value === 'string' ? field.entity.data.value : field.entity.name
    return truncatePreview(value)
  }
  return truncatePreview(field.entity.name)
}

// ── Actions ──────────────────────────────────────────────────────────────────

const enterShareMode = (stepId: string, fields: ShareableField[]): void => {
  const allKeys = Collections.toSetBy(fields, (f) => f.key)
  store.setState({
    active: true,
    sourceStepId: stepId,
    availableFields: fields,
    selectedKeys: allKeys,
    pendingChatFocus: null,
  })
}

const toggleField = (key: string): void => {
  const { selectedKeys } = store.getState()
  const next = new Set(selectedKeys)
  if (next.has(key)) {
    next.delete(key)
  } else {
    next.add(key)
  }
  store.setState({ selectedKeys: next })
}

const commitShare = (targetStepId: string): void => {
  const { active, sourceStepId, availableFields, selectedKeys } = store.getState()
  if (!active || !sourceStepId || sourceStepId === targetStepId) return

  for (const field of availableFields) {
    if (selectedKeys.has(field.key)) {
      contextMentionStore.addMention(targetStepId, field.entity, field.color, {
        chipKey: field.chipKey,
        chipPreview: deriveChipPreview(field),
      })
    }
  }

  store.setState({
    ...INITIAL_STATE,
    pendingChatFocus: targetStepId,
  })
}

const cancelShare = (): void => {
  store.setState({ ...INITIAL_STATE })
}

const clearPendingChatFocus = (): void => {
  store.setState({ pendingChatFocus: null })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const shareStore = {
  store,
  selectActive,
  selectSourceStepId,
  selectAvailableFields,
  selectSelectedKeys,
  selectPendingChatFocus,
  enterShareMode,
  toggleField,
  commitShare,
  cancelShare,
  clearPendingChatFocus,
}

export type { ShareableField, ShareState }

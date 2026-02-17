import { Archetype } from './archetypes'
import type { Archetype as ArchetypeType } from './archetypes'

type ResolveSubtitleParams = {
  archetype: ArchetypeType
  rosterNames: readonly string[]
  documentNames: readonly string[]
  roomMemberNames: readonly string[]
  parentStepName?: string | null
}

const resolveSubtitle = ({
  archetype,
  rosterNames,
  documentNames,
  roomMemberNames,
  parentStepName,
}: ResolveSubtitleParams): string | null => {
  if (archetype === Archetype.AGENT) return parentStepName ?? null
  if (archetype === Archetype.WORKFORCE) {
    const parts = [...rosterNames, ...documentNames]
    return parts.length > 0 ? parts.join(' \u00b7 ') : null
  }
  if (archetype === Archetype.ROOM && roomMemberNames.length > 0) {
    return roomMemberNames.join(' \u00b7 ')
  }
  return null
}

export { resolveSubtitle }
export type { ResolveSubtitleParams }

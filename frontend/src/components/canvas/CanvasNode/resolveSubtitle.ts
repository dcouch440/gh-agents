import { Archetype } from './registry'
import type { Archetype as ArchetypeType } from './registry'

type ResolveSubtitleParams = {
  archetype: ArchetypeType
  rosterNames: readonly string[]
  roomMemberNames: readonly string[]
  parentStepName?: string | null
}

const resolveSubtitle = ({
  archetype,
  rosterNames,
  roomMemberNames,
  parentStepName,
}: ResolveSubtitleParams): string | null => {
  if (archetype === Archetype.AGENT) return parentStepName ?? null
  if (archetype === Archetype.WORKFORCE) {
    return rosterNames.length > 0 ? rosterNames.join(' \u00b7 ') : null
  }
  if (archetype === Archetype.ROOM && roomMemberNames.length > 0) {
    return roomMemberNames.join(' \u00b7 ')
  }
  return null
}

export { resolveSubtitle }
export type { ResolveSubtitleParams }

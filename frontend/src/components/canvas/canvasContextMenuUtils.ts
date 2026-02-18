import { Collections } from '@/utils/collections'
import type { StepProtocolLink } from '@/stores'

/** Build a protocol-type lookup from canvas step-protocol links. */
const buildProtocolsByStep = (
  stepProtocols: Readonly<Record<string, StepProtocolLink>>,
): ReadonlyMap<string, { protocol_type: string }> =>
  Collections.toLookupMap(
    Object.entries(stepProtocols),
    ([sid]) => sid,
    ([, link]) => ({ protocol_type: link.protocolType }),
  )

export { buildProtocolsByStep }

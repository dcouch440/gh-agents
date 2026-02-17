import { Collections } from '@/utils/collections'
import { resolveArchetype, ARCHETYPE_CONFIGS } from './DynamicNode/archetypes'
import type { ShareableField } from '@/stores/shareStore'
import type { StepProtocolLink } from '@/stores'
import type { WorkflowStep, DocumentDef } from '@/types/workflow'

const DOC_ARTIFACT_PREFIX = 'doc-artifact-'

const parseDocArtifactId = (nodeId: string): string | null =>
  nodeId.startsWith(DOC_ARTIFACT_PREFIX) ? nodeId.slice(DOC_ARTIFACT_PREFIX.length) : null

const findParentStepForDef = (
  documentDefsByStep: Record<string, ReadonlyArray<DocumentDef>>,
  defId: string,
): string | null => {
  for (const [stepId, defs] of Object.entries(documentDefsByStep)) {
    for (let i = 0; i < defs.length; i++) {
      if (defs[i]!.id === defId) return stepId
    }
  }
  return null
}

/** Build a protocol-type lookup from canvas step-protocol links. */
const buildProtocolsByStep = (
  stepProtocols: Readonly<Record<string, StepProtocolLink>>,
): ReadonlyMap<string, { protocol_type: string }> =>
  Collections.toLookupMap(
    Object.entries(stepProtocols),
    ([sid]) => sid,
    ([, link]) => ({ protocol_type: link.protocolType }),
  )

/** Build shareable fields for a document artifact node. */
const buildDocArtifactShareFields = (
  defId: string,
  parentStep: WorkflowStep,
  parentStepId: string,
  targetDef: DocumentDef,
  protocolsByStep: ReadonlyMap<string, { protocol_type: string }>,
): ShareableField[] => {
  const archetype = resolveArchetype(parentStep, protocolsByStep, parentStepId)
  const config = ARCHETYPE_CONFIGS[archetype]
  const stepName = parentStep.name ?? 'Unnamed'

  return [
    {
      key: `doc::${targetDef.id}`,
      label: targetDef.name,
      category: 'Documents',
      kind: 'document',
      color: config.color,
      chipKey: 'doc',
      entity: {
        kind: 'document',
        id: `${parentStepId}::doc::${targetDef.id}`,
        name: targetDef.name,
        summary: `Document from ${stepName}`,
        data: { parentStepName: stepName, description: targetDef.description },
      },
    },
  ]
}

export { parseDocArtifactId, findParentStepForDef, buildProtocolsByStep, buildDocArtifactShareFields }

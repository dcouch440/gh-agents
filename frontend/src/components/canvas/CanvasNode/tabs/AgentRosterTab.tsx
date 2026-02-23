import { RosterList } from './RosterList'
import { RosterTreeView } from './RosterTreeView'

type AgentRosterTabProps = {
  stepId: string
}

function AgentRosterTab({ stepId }: AgentRosterTabProps) {
  return (
    <>
      <RosterTreeView stepId={stepId} />
      <RosterList stepId={stepId} entityLabel="Agent" />
    </>
  )
}

export { AgentRosterTab }
export type { AgentRosterTabProps }

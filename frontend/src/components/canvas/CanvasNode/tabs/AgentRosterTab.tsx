import { RosterList } from './RosterList'

type AgentRosterTabProps = {
  stepId: string
}

function AgentRosterTab({ stepId }: AgentRosterTabProps) {
  return <RosterList stepId={stepId} entityLabel="Agent" />
}

export { AgentRosterTab }
export type { AgentRosterTabProps }

import { AgentProvider, TaskProvider, PipelineProvider, FeedProvider, StatsProvider, RoutingProvider } from './contexts'
import { DashboardPage } from './pages/Dashboard/DashboardPage'
import { AgentsPage } from './pages/Agents/AgentsPage'
import { AgentDetailPage } from './pages/Agents/AgentDetailPage'
import { PipelinesPage } from './pages/Pipelines/PipelinesPage'
import { PipelineDetailPage } from './pages/Pipelines/PipelineDetailPage'
import { PipelineRunPage } from './pages/Pipelines/PipelineRunPage'
import { TasksPage } from './pages/Tasks/TasksPage'

function DashboardWithProviders() {
  return (
    <FeedProvider>
      <StatsProvider>
        <RoutingProvider>
          <DashboardPage />
        </RoutingProvider>
      </StatsProvider>
    </FeedProvider>
  )
}

function AgentsWithProvider() {
  return (
    <AgentProvider>
      <AgentsPage />
    </AgentProvider>
  )
}

function AgentDetailWithProvider() {
  return (
    <AgentProvider>
      <AgentDetailPage />
    </AgentProvider>
  )
}

function TasksWithProvider() {
  return (
    <TaskProvider>
      <TasksPage />
    </TaskProvider>
  )
}

function PipelinesWithProvider() {
  return (
    <PipelineProvider>
      <PipelinesPage />
    </PipelineProvider>
  )
}

function PipelineDetailWithProvider() {
  return (
    <PipelineProvider>
      <PipelineDetailPage />
    </PipelineProvider>
  )
}

function PipelineRunWithProvider() {
  return (
    <PipelineProvider>
      <PipelineRunPage />
    </PipelineProvider>
  )
}

export {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentDetailWithProvider,
  TasksWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
}

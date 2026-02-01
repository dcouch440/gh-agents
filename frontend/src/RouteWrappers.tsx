import { AgentProvider, TaskProvider, PipelineProvider, FeedProvider, StatsProvider, RoutingProvider, ChatProvider } from './contexts'
import { DashboardPage } from './pages/Dashboard/DashboardPage'
import { AgentsPage } from './pages/Agents/AgentsPage'
import { AgentDetailPage } from './pages/Agents/AgentDetailPage'
import { ChatPage } from './pages/Chat/ChatPage'
import { ChatSessionPage } from './pages/Chat/ChatSessionPage'
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

function ChatWithProvider() {
  return (
    <ChatProvider>
      <ChatPage />
    </ChatProvider>
  )
}

function ChatSessionWithProvider() {
  return (
    <ChatProvider>
      <ChatSessionPage />
    </ChatProvider>
  )
}

export {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentDetailWithProvider,
  ChatWithProvider,
  ChatSessionWithProvider,
  TasksWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
}

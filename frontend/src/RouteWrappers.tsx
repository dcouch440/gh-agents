import { AgentProvider, TaskProvider, FeedProvider, ChatProvider } from './contexts'
import { DashboardPage } from './pages/Dashboard/DashboardPage'
import { AgentsPage } from './pages/Agents/AgentsPage'
import { AgentDetailPage } from './pages/Agents/AgentDetailPage'
import { AgentWorkshopPage } from './pages/Agents/AgentWorkshopPage'
import { ChatPage } from './pages/Chat/ChatPage'
import { ChatSessionPage } from './pages/Chat/ChatSessionPage'
import { PipelinesPage } from './pages/Pipelines/PipelinesPage'
import { PipelineDetailPage } from './pages/Pipelines/PipelineDetailPage'
import { PipelineRunPage } from './pages/Pipelines/PipelineRunPage'
import { TasksPage } from './pages/Tasks/TasksPage'

function DashboardWithProviders() {
  return (
    <FeedProvider>
      <DashboardPage />
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

function AgentWorkshopWithProvider() {
  return (
    <AgentProvider>
      <AgentWorkshopPage />
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
  return <PipelinesPage />
}

function PipelineDetailWithProvider() {
  return <PipelineDetailPage />
}

function PipelineRunWithProvider() {
  return <PipelineRunPage />
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
  AgentWorkshopWithProvider,
  AgentDetailWithProvider,
  ChatWithProvider,
  ChatSessionWithProvider,
  TasksWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
}

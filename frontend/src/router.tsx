import { createBrowserRouter } from 'react-router-dom'
import { ROUTES } from './constants'
import { AppLayout } from './components/layout/AppLayout'
import { DashboardPage } from './pages/Dashboard/DashboardPage'
import { ChatPage } from './pages/Chat/ChatPage'
import { ChatSessionPage } from './pages/Chat/ChatSessionPage'
import { AgentsPage } from './pages/Agents/AgentsPage'
import { AgentDetailPage } from './pages/Agents/AgentDetailPage'
import { PipelinesPage } from './pages/Pipelines/PipelinesPage'
import { PipelineDetailPage } from './pages/Pipelines/PipelineDetailPage'
import { PipelineRunPage } from './pages/Pipelines/PipelineRunPage'
import { TasksPage } from './pages/Tasks/TasksPage'
import { DocumentsPage } from './pages/Documents/DocumentsPage'
import { SettingsPage } from './pages/Settings/SettingsPage'

export const router = createBrowserRouter([
  {
    element: <AppLayout />,
    children: [
      { path: ROUTES.DASHBOARD, element: <DashboardPage /> },
      { path: ROUTES.CHAT, element: <ChatPage /> },
      { path: ROUTES.CHAT_SESSION, element: <ChatSessionPage /> },
      { path: ROUTES.AGENTS, element: <AgentsPage /> },
      { path: ROUTES.AGENT_DETAIL, element: <AgentDetailPage /> },
      { path: ROUTES.PIPELINES, element: <PipelinesPage /> },
      { path: ROUTES.PIPELINE_DETAIL, element: <PipelineDetailPage /> },
      { path: ROUTES.PIPELINE_RUN, element: <PipelineRunPage /> },
      { path: ROUTES.TASKS, element: <TasksPage /> },
      { path: ROUTES.DOCUMENTS, element: <DocumentsPage /> },
      { path: ROUTES.SETTINGS, element: <SettingsPage /> },
    ],
  },
])

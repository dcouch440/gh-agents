import { createBrowserRouter } from 'react-router-dom'
import { ROUTES } from './constants'
import { AppLayout } from './components/layout/AppLayout'
import { ChatPage } from './pages/Chat/ChatPage'
import { ChatSessionPage } from './pages/Chat/ChatSessionPage'
import { DocumentsPage } from './pages/Documents/DocumentsPage'
import { SettingsPage } from './pages/Settings/SettingsPage'
import {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentDetailWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
  TasksWithProvider,
} from './RouteWrappers'

export const router = createBrowserRouter([
  {
    element: <AppLayout />,
    children: [
      { path: ROUTES.DASHBOARD, element: <DashboardWithProviders /> },
      { path: ROUTES.CHAT, element: <ChatPage /> },
      { path: ROUTES.CHAT_SESSION, element: <ChatSessionPage /> },
      { path: ROUTES.AGENTS, element: <AgentsWithProvider /> },
      { path: ROUTES.AGENT_DETAIL, element: <AgentDetailWithProvider /> },
      { path: ROUTES.PIPELINES, element: <PipelinesWithProvider /> },
      { path: ROUTES.PIPELINE_DETAIL, element: <PipelineDetailWithProvider /> },
      { path: ROUTES.PIPELINE_RUN, element: <PipelineRunWithProvider /> },
      { path: ROUTES.TASKS, element: <TasksWithProvider /> },
      { path: ROUTES.DOCUMENTS, element: <DocumentsPage /> },
      { path: ROUTES.SETTINGS, element: <SettingsPage /> },
    ],
  },
])

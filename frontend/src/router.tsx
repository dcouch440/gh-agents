import { createBrowserRouter } from 'react-router-dom'
import { ROUTES } from './constants'
import { AuthGuard } from './components/auth/AuthGuard'
import { AppLayout } from './components/layout/AppLayout'
import { DashboardPage } from './pages/Dashboard/DashboardPage'
import { AgentsPage } from './pages/Agents/AgentsPage'
import { AgentDetailPage } from './pages/Agents/AgentDetailPage'
import { AgentWorkshopPage } from './pages/Agents/AgentWorkshopPage'
import { TasksPage } from './pages/Tasks/TasksPage'
import { DocumentsPage } from './pages/Documents/DocumentsPage'
import { SettingsPage } from './pages/Settings/SettingsPage'
import { LoginPage } from './pages/Auth/LoginPage'
import { ChatPage } from './pages/Chat/ChatPage'
import { ReviewQueuePage } from './pages/ReviewQueue'
import { WorkflowsPage } from './pages/Workflows/WorkflowsPage'
import { WorkflowEditorPage } from './pages/Workflows/WorkflowEditorPage'
import { RunHistoryPage } from './pages/Workflows/RunHistoryPage'
import { RunDetailPage } from './pages/Workflows/RunDetailPage'

export const router = createBrowserRouter([
  {
    path: ROUTES.LOGIN,
    element: <LoginPage />,
  },
  {
    element: <AuthGuard />,
    children: [
      {
        element: <AppLayout />,
        children: [
          { path: ROUTES.DASHBOARD, element: <DashboardPage /> },
          { path: ROUTES.CHAT, element: <ChatPage /> },
          { path: ROUTES.AGENTS, element: <AgentsPage /> },
          { path: ROUTES.AGENT_WORKSHOP, element: <AgentWorkshopPage /> },
          { path: ROUTES.AGENT_DETAIL, element: <AgentDetailPage /> },
          { path: ROUTES.TASKS, element: <TasksPage /> },
          { path: ROUTES.WORKFLOWS, element: <WorkflowsPage /> },
          { path: ROUTES.WORKFLOW_EDITOR, element: <WorkflowEditorPage /> },
          { path: ROUTES.WORKFLOW_RUNS, element: <RunHistoryPage /> },
          { path: ROUTES.WORKFLOW_RUN_DETAIL, element: <RunDetailPage /> },
          { path: ROUTES.REVIEW_QUEUE, element: <ReviewQueuePage /> },
          { path: ROUTES.DOCUMENTS, element: <DocumentsPage /> },
          { path: ROUTES.SETTINGS, element: <SettingsPage /> },
        ],
      },
    ],
  },
])

import { createBrowserRouter } from "react-router-dom";
import { ROUTES } from "./constants";
import { AppLayout } from "./components/layout/AppLayout";
import { DashboardPage } from "./pages/Dashboard/DashboardPage";
import { AgentsPage } from "./pages/Agents/AgentsPage";
import { AgentDetailPage } from "./pages/Agents/AgentDetailPage";
import { AgentWorkshopPage } from "./pages/Agents/AgentWorkshopPage";
import { TasksPage } from "./pages/Tasks/TasksPage";
import { DocumentsPage } from "./pages/Documents/DocumentsPage";
import { SettingsPage } from "./pages/Settings/SettingsPage";
import { LoginPage } from "./pages/Auth/LoginPage";
import { ChatPage } from "./pages/Chat/ChatPage";
import { ReviewQueuePage } from "./pages/ReviewQueue";

export const router = createBrowserRouter([
  {
    path: ROUTES.LOGIN,
    element: <LoginPage />,
  },
  {
    element: <AppLayout />,
    children: [
      { path: ROUTES.DASHBOARD, element: <DashboardPage /> },
      { path: ROUTES.CHAT, element: <ChatPage /> },
      { path: ROUTES.AGENTS, element: <AgentsPage /> },
      { path: ROUTES.AGENT_WORKSHOP, element: <AgentWorkshopPage /> },
      { path: ROUTES.AGENT_DETAIL, element: <AgentDetailPage /> },
      { path: ROUTES.TASKS, element: <TasksPage /> },
      { path: ROUTES.REVIEW_QUEUE, element: <ReviewQueuePage /> },
      { path: ROUTES.DOCUMENTS, element: <DocumentsPage /> },
      { path: ROUTES.SETTINGS, element: <SettingsPage /> },
    ],
  },
]);

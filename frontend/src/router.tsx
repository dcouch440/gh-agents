import {createBrowserRouter} from "react-router-dom";
import {ROUTES} from "./constants";
import {AppLayout} from "./components/layout/AppLayout";
import {DocumentsPage} from "./pages/Documents/DocumentsPage";
import {SettingsPage} from "./pages/Settings/SettingsPage";
import {LoginPage} from "./pages/Auth/LoginPage";
import {ChatPage} from "./pages/Chat/ChatPage";
import {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentWorkshopWithProvider,
  AgentDetailWithProvider,
  TasksWithProvider,
} from "./RouteWrappers";
import {ReviewQueuePage} from "./pages/ReviewQueue";

export const router = createBrowserRouter([
  {
    path: ROUTES.LOGIN,
    element: <LoginPage />,
  },
  {
    element: <AppLayout />,
    children: [
      {path: ROUTES.DASHBOARD, element: <DashboardWithProviders />},
      {path: ROUTES.CHAT, element: <ChatPage />},
      {path: ROUTES.AGENTS, element: <AgentsWithProvider />},
      {path: ROUTES.AGENT_WORKSHOP, element: <AgentWorkshopWithProvider />},
      {path: ROUTES.AGENT_DETAIL, element: <AgentDetailWithProvider />},
      {path: ROUTES.TASKS, element: <TasksWithProvider />},
      {path: ROUTES.REVIEW_QUEUE, element: <ReviewQueuePage />},
      {path: ROUTES.DOCUMENTS, element: <DocumentsPage />},
      {path: ROUTES.SETTINGS, element: <SettingsPage />},
    ],
  },
]);

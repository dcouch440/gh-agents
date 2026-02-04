import {createBrowserRouter} from "react-router-dom";
import {ROUTES} from "./constants";
import {AppLayout} from "./components/layout/AppLayout";
import {DocumentsPage} from "./pages/Documents/DocumentsPage";
import {SettingsPage} from "./pages/Settings/SettingsPage";
import {LoginPage} from "./pages/Auth/LoginPage";
import {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentWorkshopWithProvider,
  AgentDetailWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
  TasksWithProvider,
} from "./RouteWrappers";

export const router = createBrowserRouter([
  {
    path: ROUTES.LOGIN,
    element: <LoginPage />,
  },
  {
    element: <AppLayout />,
    children: [
      {path: ROUTES.DASHBOARD, element: <DashboardWithProviders />},
      {path: ROUTES.AGENTS, element: <AgentsWithProvider />},
      {path: ROUTES.AGENT_WORKSHOP, element: <AgentWorkshopWithProvider />},
      {path: ROUTES.AGENT_DETAIL, element: <AgentDetailWithProvider />},
      {path: ROUTES.PIPELINES, element: <PipelinesWithProvider />},
      {path: ROUTES.PIPELINE_DETAIL, element: <PipelineDetailWithProvider />},
      {path: ROUTES.PIPELINE_RUN, element: <PipelineRunWithProvider />},
      {path: ROUTES.TASKS, element: <TasksWithProvider />},
      {path: ROUTES.DOCUMENTS, element: <DocumentsPage />},
      {path: ROUTES.SETTINGS, element: <SettingsPage />},
    ],
  },
]);

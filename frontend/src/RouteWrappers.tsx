import {
  AgentProvider,
  TaskProvider,
  FeedProvider,
} from "./contexts";
import {DashboardPage} from "./pages/Dashboard/DashboardPage";
import {AgentsPage} from "./pages/Agents/AgentsPage";
import {AgentDetailPage} from "./pages/Agents/AgentDetailPage";
import {AgentWorkshopPage} from "./pages/Agents/AgentWorkshopPage";
import {TasksPage} from "./pages/Tasks/TasksPage";

function DashboardWithProviders() {
  return (
    <FeedProvider>
      <DashboardPage />
    </FeedProvider>
  );
}

function AgentsWithProvider() {
  return (
    <AgentProvider>
      <AgentsPage />
    </AgentProvider>
  );
}

function AgentWorkshopWithProvider() {
  return (
    <AgentProvider>
      <AgentWorkshopPage />
    </AgentProvider>
  );
}

function AgentDetailWithProvider() {
  return (
    <AgentProvider>
      <AgentDetailPage />
    </AgentProvider>
  );
}

function TasksWithProvider() {
  return (
    <TaskProvider>
      <TasksPage />
    </TaskProvider>
  );
}

export {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentWorkshopWithProvider,
  AgentDetailWithProvider,
  TasksWithProvider,
};

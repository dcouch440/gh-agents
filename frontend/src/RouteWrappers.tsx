import {
  AgentProvider,
  TaskProvider,
  FeedProvider,
  ChatProvider,
} from "./contexts";
import {DashboardPage} from "./pages/Dashboard/DashboardPage";
import {AgentsPage} from "./pages/Agents/AgentsPage";
import {AgentDetailPage} from "./pages/Agents/AgentDetailPage";
import {AgentWorkshopPage} from "./pages/Agents/AgentWorkshopPage";
import {ChatSessionPage} from "./pages/Chat/ChatSessionPage";
import {PipelinesPage} from "./pages/Pipelines/PipelinesPage";
import {PipelineDetailPage} from "./pages/Pipelines/PipelineDetailPage";
import {PipelineRunPage} from "./pages/Pipelines/PipelineRunPage";
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

function PipelinesWithProvider() {
  return <PipelinesPage />;
}

function PipelineDetailWithProvider() {
  return <PipelineDetailPage />;
}

function PipelineRunWithProvider() {
  return <PipelineRunPage />;
}

export {
  DashboardWithProviders,
  AgentsWithProvider,
  AgentWorkshopWithProvider,
  AgentDetailWithProvider,
  TasksWithProvider,
  PipelinesWithProvider,
  PipelineDetailWithProvider,
  PipelineRunWithProvider,
};

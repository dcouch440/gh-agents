import { ListTodo } from 'lucide-react';
import { EmptyState } from '../components/EmptyState';

export function TasksPage() {
  return <EmptyState icon={ListTodo} title="Tasks" subtitle="Task management is coming soon. Track and manage agent work items here." />;
}

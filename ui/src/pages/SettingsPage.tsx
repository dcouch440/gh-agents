import { Settings } from 'lucide-react';
import { EmptyState } from '../components/EmptyState';

export function SettingsPage() {
  return <EmptyState icon={Settings} title="Settings" subtitle="Configuration is coming soon. Manage LLM providers, API keys, and preferences here." />;
}

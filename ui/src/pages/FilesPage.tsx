import { FolderOpen } from 'lucide-react';
import { EmptyState } from '../components/EmptyState';

export function FilesPage() {
  return <EmptyState icon={FolderOpen} title="Files" subtitle="File browser is coming soon. Browse and manage repository files here." />;
}

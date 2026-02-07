import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { FadeIn } from '@/components/animation';
import { PageHeader } from '@/components/primitives';

function WorkflowsPage() {
  return (
    <FadeIn>
      <PageHeader title="Workflows" description="Manage and run workflow pipelines." />
      <Box sx={{ mt: 2 }}>
        <Typography variant="body2" color="text.secondary">
          Workflow list coming soon.
        </Typography>
      </Box>
    </FadeIn>
  );
}

export { WorkflowsPage };

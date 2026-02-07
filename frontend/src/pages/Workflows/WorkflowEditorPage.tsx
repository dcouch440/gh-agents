import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { FadeIn } from '@/components/animation';

function WorkflowEditorPage() {
  return (
    <FadeIn>
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh' }}>
        <Typography variant="body2" color="text.secondary">
          Workflow editor (React Flow) coming soon.
        </Typography>
      </Box>
    </FadeIn>
  );
}

export { WorkflowEditorPage };

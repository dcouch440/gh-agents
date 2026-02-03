import { Box, Container } from '@mui/material';
import { Outlet } from 'react-router-dom';
import { TopNavBar } from './TopNavBar';

// Base-level constant: Layout spacing configuration
const LAYOUT_SPACING = {
  topPadding: 10,    // 80px (AppBar 64px + 16px margin)
  horizontalPadding: 3,
  bottomPadding: 3,
} as const;

// Stateless component - pure function
function AppLayout() {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: '100vh' }}>
      <TopNavBar />

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          pt: LAYOUT_SPACING.topPadding,
          px: LAYOUT_SPACING.horizontalPadding,
          pb: LAYOUT_SPACING.bottomPadding,
        }}
      >
        <Container maxWidth="xl" disableGutters>
          <Outlet />
        </Container>
      </Box>
    </Box>
  );
}

export { AppLayout };

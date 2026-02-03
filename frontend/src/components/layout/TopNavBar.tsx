import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import { Link as RouterLink } from 'react-router-dom';
import { useNavigation } from '@/hooks/useNavigation';
import type { NavItem } from '@/hooks/useNavigation';

// Base-level helper: Render single nav item
const renderNavItem = (item: NavItem & { isActive: boolean }) => (
  <Button
    key={item.path}
    component={RouterLink}
    to={item.path}
    color={item.isActive ? 'primary' : 'inherit'}
    sx={{
      fontWeight: item.isActive ? 600 : 400,
    }}
  >
    {item.label}
  </Button>
);

// Stateless component - pure function
function TopNavBar() {
  const { navItems } = useNavigation();

  return (
    <AppBar position="fixed" color="default">
      <Toolbar>
        <Typography
          variant="h6"
          component="div"
          sx={{ mr: 4, fontWeight: 600 }}
        >
          nexor
        </Typography>

        <Box sx={{ display: 'flex', gap: 1, flexGrow: 1 }}>
          {navItems.map(renderNavItem)}
        </Box>

        <Box sx={{ ml: 'auto' }}>
          {/* TODO: Add user menu dropdown */}
        </Box>
      </Toolbar>
    </AppBar>
  );
}

export { TopNavBar };

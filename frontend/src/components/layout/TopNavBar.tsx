import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Badge from '@mui/material/Badge';
import Box from '@mui/material/Box';
import { Link as RouterLink } from 'react-router-dom';
import { useNavigation } from '@/hooks/useNavigation';
import { useReviewQueue } from '@/hooks/useReviewQueue';
import { ROUTES } from '@/constants';
import type { NavItem } from '@/hooks/useNavigation';

type NavItemWithActive = NavItem & { isActive: boolean };

// Base-level helper: Render single nav item
const renderNavButton = (item: NavItemWithActive) => (
  <Button
    key={item.path}
    component={RouterLink}
    to={item.path}
    variant="text"
    color="inherit"
    sx={{
      fontSize: '0.9375rem',
      fontWeight: item.isActive ? 600 : 400,
      color: item.isActive ? 'primary.main' : 'text.primary',
      textTransform: 'none',
      padding: '6px 16px',
      minWidth: 'auto',
      '&:hover': {
        backgroundColor: 'rgba(255, 255, 255, 0.08)',
        color: item.isActive ? 'primary.main' : 'primary.light',
      },
    }}
  >
    {item.label}
  </Button>
);

function TopNavBar() {
  const { navItems } = useNavigation();
  const { pendingCount } = useReviewQueue();

  return (
    <AppBar position="fixed" color="default">
      <Toolbar sx={{ minHeight: 56, py: 0 }}>
        <Typography
          variant="h6"
          component="div"
          sx={{
            mr: 5,
            fontWeight: 700,
            fontSize: '1.25rem',
            letterSpacing: '-0.02em',
          }}
        >
          nexor
        </Typography>

        <Box sx={{ display: 'flex', gap: 0.5, flexGrow: 1 }}>
          {navItems.map((item) => {
            if (item.path === ROUTES.REVIEW_QUEUE && pendingCount > 0) {
              return (
                <Badge
                  key={item.path}
                  badgeContent={pendingCount}
                  color="warning"
                  sx={{ '& .MuiBadge-badge': { fontSize: '0.7rem', minWidth: 18, height: 18 } }}
                >
                  {renderNavButton(item)}
                </Badge>
              );
            }
            return renderNavButton(item);
          })}
        </Box>

        <Box sx={{ ml: 'auto' }}>
          {/* TODO: Add user menu dropdown */}
        </Box>
      </Toolbar>
    </AppBar>
  );
}

export { TopNavBar };

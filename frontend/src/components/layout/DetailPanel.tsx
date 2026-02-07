import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import CloseOutlined from '@mui/icons-material/CloseOutlined';
import { LAYOUT, ANIMATION } from '@/constants';

type DetailPanelProps = {
  side: 'left' | 'right';
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
};

function DetailPanel({ side, isOpen, onClose, title, children }: DetailPanelProps) {
  const isLeft = side === 'left';

  return (
    <Box
      sx={{
        width: isOpen ? LAYOUT.PANEL_WIDTH : 0,
        minWidth: isOpen ? LAYOUT.PANEL_WIDTH : 0,
        height: '100vh',
        overflow: 'hidden',
        transition: `all ${ANIMATION.NORMAL}ms ease`,
        borderRight: isLeft ? 1 : 0,
        borderLeft: isLeft ? 0 : 1,
        borderColor: isOpen ? 'divider' : 'transparent',
        backgroundColor: 'background.paper',
        backdropFilter: 'blur(12px)',
        display: 'flex',
        flexDirection: 'column',
        position: 'sticky',
        top: 0,
      }}
    >
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          px: 2,
          py: 1.5,
          minHeight: 48,
          borderBottom: 1,
          borderColor: 'divider',
          opacity: isOpen ? 1 : 0,
          transition: `opacity ${ANIMATION.FAST}ms ease`,
        }}
      >
        <Typography
          variant="subtitle2"
          sx={{ fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden' }}
        >
          {title}
        </Typography>
        <IconButton
          onClick={onClose}
          size="small"
          sx={{
            color: 'text.secondary',
            '&:hover': { color: 'text.primary' },
          }}
        >
          <CloseOutlined sx={{ fontSize: 18 }} />
        </IconButton>
      </Box>

      {/* Content */}
      <Box
        sx={{
          flexGrow: 1,
          overflow: 'auto',
          px: 2,
          py: 1.5,
          opacity: isOpen ? 1 : 0,
          transition: `opacity ${ANIMATION.FAST}ms ease`,
        }}
      >
        {children}
      </Box>
    </Box>
  );
}

export { DetailPanel };
export type { DetailPanelProps };

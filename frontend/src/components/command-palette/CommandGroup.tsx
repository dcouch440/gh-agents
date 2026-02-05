import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';

type CommandGroupProps = {
  label: string;
  children: ReactNode;
};

function CommandGroup({ label, children }: CommandGroupProps) {
  return (
    <Box sx={{ mb: 0.5 }}>
      <Typography
        variant="overline"
        sx={{
          px: 2,
          py: 0.5,
          display: 'block',
          color: 'text.secondary',
          fontSize: '0.65rem',
          letterSpacing: '0.08em',
        }}
      >
        {label}
      </Typography>
      {children}
    </Box>
  );
}

export { CommandGroup };

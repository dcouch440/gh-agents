import { type ReactNode } from 'react';
import Dialog from '@mui/material/Dialog';

type CommandDialogProps = {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  maxWidth?: number;
};

function CommandDialog({ open, onClose, children, maxWidth = 580 }: CommandDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth={false}
      sx={{
        '& .MuiDialog-container': {
          alignItems: 'flex-start',
          pt: '15vh',
        },
        '& .MuiDialog-paper': {
          width: '100%',
          maxWidth,
          borderRadius: 2,
          overflow: 'hidden',
        },
        '& .MuiBackdrop-root': {
          backdropFilter: 'blur(4px)',
        },
      }}
    >
      {children}
    </Dialog>
  );
}

export { CommandDialog };

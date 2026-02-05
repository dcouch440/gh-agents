import { type ReactNode } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { ANIMATION } from '@/constants';

type CommandItemProps = {
  icon?: ReactNode;
  label: string;
  description?: string;
  shortcut?: string;
  selected: boolean;
  onSelect: () => void;
};

function CommandItemRow({ icon, label, description, shortcut, selected, onSelect }: CommandItemProps) {
  return (
    <Box
      onClick={onSelect}
      role="option"
      aria-selected={selected}
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        py: 1,
        cursor: 'pointer',
        borderRadius: 1,
        mx: 1,
        transition: `background-color ${ANIMATION.FAST}ms ease`,
        backgroundColor: selected ? 'action.selected' : 'transparent',
        '&:hover': {
          backgroundColor: 'action.hover',
        },
      }}
    >
      {icon && (
        <Box sx={{ color: 'text.secondary', display: 'flex', alignItems: 'center', flexShrink: 0 }}>
          {icon}
        </Box>
      )}

      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Typography variant="body2" sx={{ fontWeight: 500, color: 'text.primary' }} noWrap>
          {label}
        </Typography>
        {description && (
          <Typography variant="caption" sx={{ color: 'text.secondary' }} noWrap>
            {description}
          </Typography>
        )}
      </Box>

      {shortcut && (
        <Typography
          variant="caption"
          sx={{
            color: 'text.secondary',
            fontFamily: 'monospace',
            fontSize: '0.7rem',
            px: 0.75,
            py: 0.25,
            borderRadius: 0.5,
            border: 1,
            borderColor: 'divider',
            flexShrink: 0,
          }}
        >
          {shortcut}
        </Typography>
      )}
    </Box>
  );
}

export { CommandItemRow };
export type { CommandItemProps };

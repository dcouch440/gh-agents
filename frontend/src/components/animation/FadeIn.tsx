import { useEffect, useState, type ReactNode } from 'react';
import Box from '@mui/material/Box';
import { ANIMATION } from '@/constants';
import { useReducedMotion } from '@/hooks/useReducedMotion';

type FadeInProps = {
  children: ReactNode;
  delay?: number;
  duration?: number;
  direction?: 'up' | 'down' | 'none';
};

const getTranslate = (direction: FadeInProps['direction'], entered: boolean): string => {
  if (entered) return 'translateY(0)';
  switch (direction) {
    case 'down': return 'translateY(-8px)';
    case 'none': return 'none';
    case 'up':
    default: return 'translateY(8px)';
  }
};

function FadeIn({ children, delay = 0, duration = ANIMATION.PAGE_TRANSITION, direction = 'up' }: FadeInProps) {
  const [entered, setEntered] = useState(false);
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    const timeout = setTimeout(() => setEntered(true), delay);
    return () => clearTimeout(timeout);
  }, [delay]);

  if (reducedMotion) {
    return <>{children}</>;
  }

  return (
    <Box
      sx={{
        opacity: entered ? 1 : 0,
        transform: getTranslate(direction, entered),
        transition: `opacity ${duration}ms ease, transform ${duration}ms ease`,
      }}
    >
      {children}
    </Box>
  );
}

export { FadeIn };

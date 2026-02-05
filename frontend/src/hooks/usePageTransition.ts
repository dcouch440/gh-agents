import { useEffect, useState } from 'react';
import type { SxProps, Theme } from '@mui/material/styles';
import { ANIMATION } from '@/constants';
import { useReducedMotion } from './useReducedMotion';

const usePageTransition = () => {
  const [entered, setEntered] = useState(false);
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    const frame = requestAnimationFrame(() => setEntered(true));
    return () => cancelAnimationFrame(frame);
  }, []);

  const transitionSx: SxProps<Theme> = reducedMotion
    ? {}
    : {
        opacity: entered ? 1 : 0,
        transform: entered ? 'translateY(0)' : 'translateY(8px)',
        transition: `opacity ${ANIMATION.PAGE_TRANSITION}ms ease, transform ${ANIMATION.PAGE_TRANSITION}ms ease`,
      };

  return { entered, transitionSx };
};

export { usePageTransition };

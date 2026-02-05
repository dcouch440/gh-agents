import type { SxProps, Theme } from '@mui/material/styles';
import { ANIMATION } from '@/constants';
import { useReducedMotion } from './useReducedMotion';

type HoverAnimationOptions = {
  lift?: number;
  shadow?: number;
};

const useHoverAnimation = (options?: HoverAnimationOptions): SxProps<Theme> => {
  const reducedMotion = useReducedMotion();

  if (reducedMotion) return {};

  const lift = options?.lift ?? 2;
  const shadow = options?.shadow ?? 2;

  return {
    transition: `transform ${ANIMATION.FAST}ms ease, box-shadow ${ANIMATION.FAST}ms ease`,
    '&:hover': {
      transform: `translateY(-${lift}px)`,
      boxShadow: shadow,
    },
  };
};

export { useHoverAnimation };

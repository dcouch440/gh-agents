import { Children, type ReactNode } from 'react'
import { FadeIn } from './FadeIn'
import { ANIMATION } from '@/constants'

type AnimatedListProps = {
  children: ReactNode
  staggerMs?: number
  duration?: number
  direction?: 'up' | 'down' | 'none'
}

function AnimatedList({ children, staggerMs = 50, duration = ANIMATION.PAGE_TRANSITION, direction = 'up' }: AnimatedListProps) {
  return (
    <>
      {Children.map(children, (child, index) => (
        <FadeIn key={index} delay={index * staggerMs} duration={duration} direction={direction}>
          {child}
        </FadeIn>
      ))}
    </>
  )
}

export { AnimatedList }

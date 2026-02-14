import { useEffect, useRef, useState } from 'react'
import type { RefObject } from 'react'
import { resolveScaleFactor } from './CanvasFormNode/scaleNotch'

type UseNodeScaleResult = {
  containerRef: RefObject<HTMLDivElement | null>
  scaleFactor: number
}

/**
 * Tracks a container's size via ResizeObserver and returns a CSS zoom
 * scale factor based on the notch breakpoints defined in CanvasFormNode.
 */
const useNodeScale = (): UseNodeScaleResult => {
  const containerRef = useRef<HTMLDivElement>(null)
  const [scaleFactor, setScaleFactor] = useState(1)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect
      if (!rect) return
      const next = resolveScaleFactor(rect.width, rect.height)
      setScaleFactor((prev) => (prev === next ? prev : next))
    })
    observer.observe(el)
    return () => { observer.disconnect() }
  }, [])

  return { containerRef, scaleFactor }
}

export { useNodeScale }

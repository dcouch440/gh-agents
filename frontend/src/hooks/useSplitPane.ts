import { useState, useCallback, useEffect, useRef } from 'react'

type UseSplitPaneOpts = {
  initial: number
  min: number
  max: number
}

type UseSplitPaneResult = {
  splitPercent: number
  handleMouseDown: (e: React.MouseEvent) => void
}

const useSplitPane = (opts: UseSplitPaneOpts): UseSplitPaneResult => {
  const [splitPercent, setSplitPercent] = useState(opts.initial)
  const draggingRef = useRef(false)
  const containerRef = useRef<HTMLElement | null>(null)
  const listenersRef = useRef<{ move: (e: MouseEvent) => void; up: () => void } | null>(null)

  const cleanup = useCallback(() => {
    if (listenersRef.current) {
      document.removeEventListener('mousemove', listenersRef.current.move)
      document.removeEventListener('mouseup', listenersRef.current.up)
      listenersRef.current = null
    }
    draggingRef.current = false
    document.body.style.userSelect = ''
  }, [])

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      cleanup()
      draggingRef.current = true
      containerRef.current = e.currentTarget.parentElement as HTMLElement
      document.body.style.userSelect = 'none'

      const move = (ev: MouseEvent) => {
        if (!containerRef.current) return
        const rect = containerRef.current.getBoundingClientRect()
        const percent = ((ev.clientX - rect.left) / rect.width) * 100
        const clamped = Math.min(opts.max, Math.max(opts.min, percent))
        setSplitPercent(clamped)
      }

      const up = () => { cleanup() }

      listenersRef.current = { move, up }
      document.addEventListener('mousemove', move)
      document.addEventListener('mouseup', up)
    },
    [opts.min, opts.max, cleanup]
  )

  useEffect(() => {
    return () => { cleanup() }
  }, [cleanup])

  return { splitPercent, handleMouseDown }
}

export { useSplitPane }
export type { UseSplitPaneOpts, UseSplitPaneResult }

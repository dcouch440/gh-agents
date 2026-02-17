import { useCallback, useState } from 'react'
import { shareStore } from '@/stores'
import type { MenuPosition } from './CanvasContextMenu'

type ScreenToFlowPosition = (position: { x: number; y: number }) => { x: number; y: number }

type ContextMenuState = {
  contextMenu: MenuPosition
  closeMenu: () => void
  onPaneContextMenu: (event: React.MouseEvent | MouseEvent) => void
  onNodeContextMenu: (event: React.MouseEvent, node: { id: string; position: { x: number; y: number } }) => void
  onCanvasMouseDown: () => void
}

const useContextMenuState = (screenToFlowPosition: ScreenToFlowPosition): ContextMenuState => {
  const [contextMenu, setContextMenu] = useState<MenuPosition>(null)

  const closeMenu = useCallback(() => {
    setContextMenu(null)
  }, [])

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault()
      if (shareStore.store.getState().active) return
      const flowPosition = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      })
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        flowX: flowPosition.x,
        flowY: flowPosition.y,
      })
    },
    [screenToFlowPosition],
  )

  const onNodeContextMenu = useCallback((event: React.MouseEvent, node: { id: string; position: { x: number; y: number } }) => {
    event.preventDefault()
    if (shareStore.store.getState().active) return
    setContextMenu({
      x: event.clientX,
      y: event.clientY,
      flowX: node.position.x,
      flowY: node.position.y,
      nodeId: node.id,
    })
  }, [])

  const onCanvasMouseDown = useCallback(() => {
    setContextMenu(null)
  }, [])

  return { contextMenu, closeMenu, onPaneContextMenu, onNodeContextMenu, onCanvasMouseDown }
}

export { useContextMenuState }

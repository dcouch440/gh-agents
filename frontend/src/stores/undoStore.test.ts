import { boardElementStore } from './boardElementStore'
import { undoStore } from './undoStore'
import type { BoxElement } from '@/components/board/elements'
import { addBox, emptyBoard } from '@/components/board/elements'

const makeBox = (id: string, x = 0, y = 0): BoxElement => ({
  id,
  type: 'box',
  x,
  y,
  width: 200,
  height: 48,
  text: '',
})

const getState = () => undoStore.store.getState()

beforeEach(() => {
  undoStore.clear()
  boardElementStore.replaceElements(emptyBoard())
})

describe('undoStore', () => {
  describe('push / undo / redo', () => {
    it('captures snapshot and restores on undo', () => {
      const before = boardElementStore.getElements()

      undoStore.push('create-box')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))

      expect(boardElementStore.getElements().boxes.has('a')).toBe(true)

      undoStore.undo()

      expect(boardElementStore.getElements()).toBe(before)
      expect(boardElementStore.getElements().boxes.has('a')).toBe(false)
    })

    it('redo restores the undone state', () => {
      undoStore.push('create-box')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      const afterAdd = boardElementStore.getElements()

      undoStore.undo()
      undoStore.redo()

      expect(boardElementStore.getElements()).toBe(afterAdd)
    })

    it('clears future on new push', () => {
      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.push('b')
      boardElementStore.setElements((s) => addBox(s, makeBox('b')))

      undoStore.undo()
      expect(getState().future).toHaveLength(1)

      undoStore.push('c')
      boardElementStore.setElements((s) => addBox(s, makeBox('c')))

      expect(getState().future).toHaveLength(0)
    })

    it('multiple undo/redo cycle', () => {
      const s0 = boardElementStore.getElements()

      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))

      undoStore.push('b')
      boardElementStore.setElements((s) => addBox(s, makeBox('b')))

      // Undo b
      undoStore.undo()
      expect(boardElementStore.getElements().boxes.has('b')).toBe(false)
      expect(boardElementStore.getElements().boxes.has('a')).toBe(true)

      // Undo a
      undoStore.undo()
      expect(boardElementStore.getElements()).toBe(s0)

      // Redo a
      undoStore.redo()
      expect(boardElementStore.getElements().boxes.has('a')).toBe(true)
      expect(boardElementStore.getElements().boxes.has('b')).toBe(false)

      // Redo b
      undoStore.redo()
      expect(boardElementStore.getElements().boxes.has('a')).toBe(true)
      expect(boardElementStore.getElements().boxes.has('b')).toBe(true)
    })
  })

  describe('no-op on empty stacks', () => {
    it('undo is a no-op when past is empty', () => {
      const before = boardElementStore.getElements()
      undoStore.undo()
      expect(boardElementStore.getElements()).toBe(before)
    })

    it('redo is a no-op when future is empty', () => {
      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      const before = boardElementStore.getElements()

      undoStore.redo()
      expect(boardElementStore.getElements()).toBe(before)
    })
  })

  describe('max depth', () => {
    it('trims past when exceeding 100 entries', () => {
      for (let i = 0; i < 105; i++) {
        undoStore.push(`item-${i}`)
        boardElementStore.setElements((s) => addBox(s, makeBox(`box-${i}`)))
      }

      expect(getState().past).toHaveLength(100)
      // Oldest entries should be trimmed
      expect(getState().past[0].tag).toBe('item-5')
    })
  })

  describe('transactions', () => {
    it('collapses multiple mutations into one undo unit', () => {
      const before = boardElementStore.getElements()

      undoStore.beginTransaction('move')
      boardElementStore.setElements((s) => addBox(s, makeBox('a', 0, 0)))
      boardElementStore.setElements((s) => addBox(s, makeBox('b', 10, 10)))
      undoStore.commit()

      expect(getState().past).toHaveLength(1)
      expect(getState().past[0].tag).toBe('move')

      undoStore.undo()
      expect(boardElementStore.getElements()).toBe(before)
    })

    it('suppresses push() calls inside a transaction', () => {
      undoStore.beginTransaction('drag')
      undoStore.push('should-be-ignored')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.commit()

      // Only the transaction entry, not the push
      expect(getState().past).toHaveLength(1)
      expect(getState().past[0].tag).toBe('drag')
    })

    it('discards transaction when state did not change', () => {
      undoStore.beginTransaction('no-op')
      undoStore.commit()

      expect(getState().past).toHaveLength(0)
    })

    it('supports nested transactions (ref-counted)', () => {
      const before = boardElementStore.getElements()

      undoStore.beginTransaction('outer')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.beginTransaction('inner')
      boardElementStore.setElements((s) => addBox(s, makeBox('b')))
      undoStore.commit() // inner
      undoStore.commit() // outer

      expect(getState().past).toHaveLength(1)
      expect(getState().past[0].tag).toBe('outer')

      undoStore.undo()
      expect(boardElementStore.getElements()).toBe(before)
    })

    it('rollback restores the snapshot and discards transaction', () => {
      const before = boardElementStore.getElements()

      undoStore.beginTransaction('drag')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.rollback()

      expect(boardElementStore.getElements()).toBe(before)
      expect(getState().past).toHaveLength(0)
      expect(getState().transactionDepth).toBe(0)
    })
  })

  describe('clear', () => {
    it('resets both stacks and transaction state', () => {
      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.push('b')
      boardElementStore.setElements((s) => addBox(s, makeBox('b')))
      undoStore.undo()

      undoStore.clear()

      expect(getState().past).toHaveLength(0)
      expect(getState().future).toHaveLength(0)
      expect(getState().transactionDepth).toBe(0)
      expect(getState().pendingEntry).toBeNull()
    })
  })

  describe('selectors', () => {
    it('selectCanUndo reflects past state', () => {
      expect(undoStore.selectCanUndo(getState())).toBe(false)

      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))

      expect(undoStore.selectCanUndo(getState())).toBe(true)
    })

    it('selectCanRedo reflects future state', () => {
      expect(undoStore.selectCanRedo(getState())).toBe(false)

      undoStore.push('a')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.undo()

      expect(undoStore.selectCanRedo(getState())).toBe(true)
    })

    it('selectUndoTag returns tag of most recent past entry', () => {
      expect(undoStore.selectUndoTag(getState())).toBeNull()

      undoStore.push('create-box')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.push('draw-arrow')
      boardElementStore.setElements((s) => addBox(s, makeBox('b')))

      expect(undoStore.selectUndoTag(getState())).toBe('draw-arrow')
    })

    it('selectRedoTag returns tag of most recent future entry', () => {
      expect(undoStore.selectRedoTag(getState())).toBeNull()

      undoStore.push('delete')
      boardElementStore.setElements((s) => addBox(s, makeBox('a')))
      undoStore.undo()

      expect(undoStore.selectRedoTag(getState())).toBe('delete')
    })
  })
})

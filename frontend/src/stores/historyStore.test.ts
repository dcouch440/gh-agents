import { historyStore } from './historyStore'
import type { Command } from './historyStore'

const getState = () => historyStore.store.getState()

const makeCommand = (type: string, description: string): Command & { executeCalls: number; undoCalls: number } => {
  const cmd = {
    type,
    description,
    executeCalls: 0,
    undoCalls: 0,
    execute: () => { cmd.executeCalls++ },
    undo: () => { cmd.undoCalls++ },
  }
  return cmd
}

beforeEach(() => {
  historyStore.clear()
  historyStore.setMaxSize(50)
})

describe('historyStore', () => {
  describe('push', () => {
    it('executes command and adds to past', () => {
      const cmd = makeCommand('test', 'Test command')

      historyStore.push(cmd)

      expect(cmd.executeCalls).toBe(1)
      expect(getState().past).toHaveLength(1)
      expect(getState().past[0]).toBe(cmd)
    })

    it('clears future on push', () => {
      const cmd1 = makeCommand('a', 'First')
      const cmd2 = makeCommand('b', 'Second')
      const cmd3 = makeCommand('c', 'Third')

      historyStore.push(cmd1)
      historyStore.push(cmd2)
      historyStore.undo()

      // Future now has cmd2
      expect(getState().future).toHaveLength(1)

      historyStore.push(cmd3)

      // Future cleared after new push
      expect(getState().future).toHaveLength(0)
      expect(getState().past).toHaveLength(2)
    })

    it('trims past when exceeding maxSize', () => {
      historyStore.setMaxSize(3)

      for (let i = 0; i < 5; i++) {
        historyStore.push(makeCommand(`cmd-${i}`, `Command ${i}`))
      }

      expect(getState().past).toHaveLength(3)
      expect(getState().past[0].description).toBe('Command 2')
      expect(getState().past[2].description).toBe('Command 4')
    })
  })

  describe('undo', () => {
    it('calls undo and moves command to future', () => {
      const cmd = makeCommand('test', 'Test')

      historyStore.push(cmd)
      historyStore.undo()

      expect(cmd.undoCalls).toBe(1)
      expect(getState().past).toHaveLength(0)
      expect(getState().future).toHaveLength(1)
      expect(getState().future[0]).toBe(cmd)
    })

    it('is a no-op when past is empty', () => {
      const before = getState()

      historyStore.undo()

      expect(getState().past).toEqual(before.past)
      expect(getState().future).toEqual(before.future)
    })
  })

  describe('redo', () => {
    it('calls execute and moves command back to past', () => {
      const cmd = makeCommand('test', 'Test')

      historyStore.push(cmd)
      historyStore.undo()
      historyStore.redo()

      // execute called on push + redo = 2
      expect(cmd.executeCalls).toBe(2)
      expect(getState().past).toHaveLength(1)
      expect(getState().future).toHaveLength(0)
    })

    it('is a no-op when future is empty', () => {
      historyStore.push(makeCommand('test', 'Test'))
      const before = getState()

      historyStore.redo()

      expect(getState().past).toBe(before.past)
      expect(getState().future).toBe(before.future)
    })
  })

  describe('clear', () => {
    it('resets both stacks', () => {
      historyStore.push(makeCommand('a', 'First'))
      historyStore.push(makeCommand('b', 'Second'))
      historyStore.undo()

      historyStore.clear()

      expect(getState().past).toHaveLength(0)
      expect(getState().future).toHaveLength(0)
    })
  })

  describe('setMaxSize', () => {
    it('trims past if current exceeds new max', () => {
      for (let i = 0; i < 5; i++) {
        historyStore.push(makeCommand(`cmd-${i}`, `Command ${i}`))
      }

      historyStore.setMaxSize(2)

      expect(getState().past).toHaveLength(2)
      expect(getState().maxSize).toBe(2)
    })
  })

  describe('selectors', () => {
    it('selectCanUndo returns true when past is non-empty', () => {
      expect(historyStore.selectCanUndo(getState())).toBe(false)

      historyStore.push(makeCommand('test', 'Test'))

      expect(historyStore.selectCanUndo(getState())).toBe(true)
    })

    it('selectCanRedo returns true when future is non-empty', () => {
      expect(historyStore.selectCanRedo(getState())).toBe(false)

      historyStore.push(makeCommand('test', 'Test'))
      historyStore.undo()

      expect(historyStore.selectCanRedo(getState())).toBe(true)
    })

    it('selectUndoDescription returns last past command description', () => {
      expect(historyStore.selectUndoDescription(getState())).toBeNull()

      historyStore.push(makeCommand('a', 'Add step'))
      historyStore.push(makeCommand('b', 'Move step'))

      expect(historyStore.selectUndoDescription(getState())).toBe('Move step')
    })

    it('selectRedoDescription returns last future command description', () => {
      expect(historyStore.selectRedoDescription(getState())).toBeNull()

      historyStore.push(makeCommand('a', 'Delete edge'))
      historyStore.undo()

      expect(historyStore.selectRedoDescription(getState())).toBe('Delete edge')
    })
  })
})

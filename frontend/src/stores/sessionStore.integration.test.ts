import { sessionStore } from './sessionStore'
import { SESSION_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import { createNormalizedMap, nmSize, nmGet } from './lib'

vi.mock('@/api', () => ({
  api: { sessions: { list: vi.fn(), get: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn() } },
}))

let tsCounter = 0
const makeMsg = (event: string, data: Record<string, unknown>): WsWireMessage => ({
  topic: 'session',
  event,
  ts: `2025-01-01T00:00:${String(tsCounter++).padStart(2, '0')}Z`,
  run_id: null,
  user_id: null,
  data,
})

const handle = sessionStore.handleWsEvent

beforeEach(() => {
  sessionStore.store.setState({
    items: createNormalizedMap(),
    loading: false,
    error: null,
    lastFetched: null,
  })
  tsCounter = 0
})

describe('sessionStore integration', () => {
  describe('create → update → delete sequence', () => {
    it('tracks state at each step', () => {
      handle(makeMsg(SESSION_EVENT.CREATED, { session_id: 'ses1', title: 'Chat 1', mode_id: 'chat' }))
      expect(nmSize(sessionStore.store.getState().items)).toBe(1)
      expect(nmGet(sessionStore.store.getState().items, 'ses1')?.title).toBe('Chat 1')

      handle(makeMsg(SESSION_EVENT.UPDATED, { session_id: 'ses1', title: 'Renamed' }))
      expect(nmGet(sessionStore.store.getState().items, 'ses1')?.title).toBe('Renamed')

      handle(makeMsg(SESSION_EVENT.DELETED, { session_id: 'ses1' }))
      expect(nmSize(sessionStore.store.getState().items)).toBe(0)
    })
  })

  describe('malformed messages', () => {
    it('malformed CREATED (missing session_id) — state unchanged', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      handle(makeMsg(SESSION_EVENT.CREATED, { title: 'No ID' }))
      spy.mockRestore()

      // Should not have added an item with undefined key — or at most not crash
      const items = sessionStore.store.getState().items
      expect(nmGet(items, 'ses1')).toBeUndefined()
    })

    it('mixed valid + malformed — only valid events applied', () => {
      handle(makeMsg(SESSION_EVENT.CREATED, { session_id: 'ses1', title: 'Good', mode_id: 'chat' }))

      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      handle(makeMsg(SESSION_EVENT.UPDATED, {}))
      spy.mockRestore()

      // ses1 should still exist and be unchanged
      expect(nmGet(sessionStore.store.getState().items, 'ses1')?.title).toBe('Good')

      handle(makeMsg(SESSION_EVENT.CREATED, { session_id: 'ses2', title: 'Also Good', mode_id: 'chat' }))
      expect(nmSize(sessionStore.store.getState().items)).toBe(2)
    })
  })

  describe('rapid create/delete', () => {
    it('handles rapid alternating creates and deletes', () => {
      for (let i = 0; i < 10; i++) {
        handle(makeMsg(SESSION_EVENT.CREATED, { session_id: `ses${i}`, title: `Session ${i}`, mode_id: 'chat' }))
      }
      expect(nmSize(sessionStore.store.getState().items)).toBe(10)

      for (let i = 0; i < 5; i++) {
        handle(makeMsg(SESSION_EVENT.DELETED, { session_id: `ses${i}` }))
      }
      expect(nmSize(sessionStore.store.getState().items)).toBe(5)

      // Remaining sessions are ses5..ses9
      for (let i = 5; i < 10; i++) {
        expect(nmGet(sessionStore.store.getState().items, `ses${i}`)).toBeDefined()
      }
    })
  })
})

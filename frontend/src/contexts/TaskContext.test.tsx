import { render, screen, waitFor } from '@testing-library/react'
import { TaskProvider } from './TaskContext'
import { useTaskContext } from '@/hooks/useTaskContext'
import { mockTask, mockTaskCompleted } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

let wsHandler: ((data: unknown) => void) | null = null

vi.mock('@/hooks/useWebSocket', () => ({
  useWebSocket: () => ({
    status: 'connected' as const,
    subscribe: (_channel: string, handler: (data: unknown) => void) => {
      wsHandler = handler
      return () => { wsHandler = null }
    },
  }),
}))

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('@/api', () => ({
  api: { get: mockGet },
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

// ── Test consumer ────────────────────────────────────────────────────────────

function TestConsumer() {
  const { tasks, loading, error } = useTaskContext()
  if (loading) return <div>loading</div>
  if (error) return <div>error: {error}</div>
  return (
    <div>
      {tasks.map((t) => (
        <div key={t.id} data-testid={`task-${t.id}`}>
          {t.title}:{t.status}
        </div>
      ))}
    </div>
  )
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('TaskContext', () => {
  describe('TaskProvider', () => {
    beforeEach(() => {
      wsHandler = null
      vi.clearAllMocks()
      mockGet.mockResolvedValue([mockTask])
    })

    it('fetches tasks on mount', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toHaveTextContent('Test task:pending')
      })
    })

    it('updates a task via WS', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toBeInTheDocument()
      })

      wsHandler?.({ task: mockTaskCompleted })

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toHaveTextContent('Test task:completed')
      })
    })

    it('removes a task via WS deleted_id', async () => {
      render(
        <TaskProvider>
          <TestConsumer />
        </TaskProvider>,
      )

      await waitFor(() => {
        expect(screen.getByTestId('task-task-001')).toBeInTheDocument()
      })

      wsHandler?.({ deleted_id: 'task-001' })

      await waitFor(() => {
        expect(screen.queryByTestId('task-task-001')).not.toBeInTheDocument()
      })
    })

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useTaskContext must be used within TaskProvider')
      spy.mockRestore()
    })
  })
})

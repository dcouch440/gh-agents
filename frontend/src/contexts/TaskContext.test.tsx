import { render, screen, waitFor } from '@testing-library/react'
import { TaskProvider } from './TaskContext'
import { useTaskContext } from '@/hooks/useTaskContext'
import { mockTask } from '@/test/fixtures'

// ── Mocks ────────────────────────────────────────────────────────────────────

const { mockList } = vi.hoisted(() => ({ mockList: vi.fn() }))

vi.mock('@/api', () => ({
  api: { tasks: { list: mockList } },
}))

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
      vi.clearAllMocks()
      mockList.mockResolvedValue({ items: [mockTask] })
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

    it('throws when hook is used outside provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      expect(() => render(<TestConsumer />)).toThrow('useTaskContext must be used within TaskProvider')
      spy.mockRestore()
    })
  })
})

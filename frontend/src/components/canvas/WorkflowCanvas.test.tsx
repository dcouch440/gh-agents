import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { WorkflowCanvas } from './WorkflowCanvas'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

const {
  mockSelectSteps,
  mockSelectEdges,
  mockClearSelection,
  mockOpenRightPanelIfClosed,
  mockAddEdge,
  mockDeleteStep,
  mockRemoveEdge,
  _steps,
  _edges,
  _minimapVisible,
  mockFitView,
  mockScreenToFlowPosition,
  mockSetNodes,
  mockSetEdges,
} = vi.hoisted(() => ({
  mockSelectSteps: vi.fn(),
  mockSelectEdges: vi.fn(),
  mockClearSelection: vi.fn(),
  mockOpenRightPanelIfClosed: vi.fn(),
  mockAddEdge: vi.fn(),
  mockDeleteStep: vi.fn(),
  mockRemoveEdge: vi.fn(),
  _steps: { value: [] as WorkflowStep[] },
  _edges: { value: [] as WorkflowStepEdge[] },
  _minimapVisible: { value: false },
  mockFitView: vi.fn(),
  mockScreenToFlowPosition: vi.fn(() => ({ x: 0, y: 0 })),
  mockSetNodes: vi.fn(),
  mockSetEdges: vi.fn(),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  workflowStore: {
    store: 'workflow',
    selectSteps: () => _steps.value,
    selectEdges: () => _edges.value,
    addEdge: mockAddEdge,
    deleteStep: mockDeleteStep,
    removeEdge: mockRemoveEdge,
    updateStep: vi.fn(),
    createStep: vi.fn(),
  },
  canvasStore: {
    store: 'canvas',
    selectMinimapVisible: () => _minimapVisible.value,
    selectSteps: mockSelectSteps,
    selectEdges: mockSelectEdges,
    clearSelection: mockClearSelection,
  },
  layoutStore: {
    store: {
      getState: () => ({ rightPanelOpen: false, rightPanelSection: null }),
    },
    openRightPanelIfClosed: mockOpenRightPanelIfClosed,
  },
  agentStore: {
    store: 'agent',
    selectAll: () => [],
  },
  outputSchemaStore: {
    store: 'outputSchema',
    selectAll: () => [],
  },
}))

// Mock React Flow — jsdom can't render SVG canvas
vi.mock('@xyflow/react', () => {
  const MockReactFlow = ({ children, onSelectionChange, onConnect, onReconnect, onNodesDelete, onEdgesDelete }: {
    children?: React.ReactNode
    onSelectionChange?: (params: { nodes: { id: string }[]; edges: { id: string }[] }) => void
    onConnect?: (connection: { source: string; target: string }) => void
    onReconnect?: (oldEdge: { id: string }, newConnection: { source: string; target: string }) => void
    onNodesDelete?: (nodes: { id: string }[]) => void
    onEdgesDelete?: (edges: { id: string }[]) => void
  }) => (
    <div
      data-testid="react-flow"
      data-on-selection-change={onSelectionChange ? 'yes' : 'no'}
      data-on-connect={onConnect ? 'yes' : 'no'}
      data-on-reconnect={onReconnect ? 'yes' : 'no'}
      data-on-nodes-delete={onNodesDelete ? 'yes' : 'no'}
      data-on-edges-delete={onEdgesDelete ? 'yes' : 'no'}
    >
      {children}
    </div>
  )

  const MockReactFlowProvider = ({ children }: { children?: React.ReactNode }) => <>{children}</>

  return {
    ReactFlow: MockReactFlow,
    ReactFlowProvider: MockReactFlowProvider,
    Background: () => <div data-testid="background" />,
    BackgroundVariant: { Dots: 'dots' },
    MiniMap: () => <div data-testid="minimap" />,
    useReactFlow: () => ({
      fitView: mockFitView,
      screenToFlowPosition: mockScreenToFlowPosition,
      setNodes: mockSetNodes,
      setEdges: mockSetEdges,
    }),
    Position: { Left: 'left', Right: 'right' },
  }
})

vi.mock('@xyflow/react/dist/style.css', () => ({}))

beforeEach(() => {
  vi.clearAllMocks()
  _steps.value = []
  _edges.value = []
  _minimapVisible.value = false
})

describe('WorkflowCanvas', () => {
  it('renders ReactFlow component', () => {
    render(<WorkflowCanvas />)
    expect(screen.getByTestId('react-flow')).toBeInTheDocument()
  })

  it('renders background', () => {
    render(<WorkflowCanvas />)
    expect(screen.getByTestId('background')).toBeInTheDocument()
  })

  it('does not render minimap by default', () => {
    render(<WorkflowCanvas />)
    expect(screen.queryByTestId('minimap')).not.toBeInTheDocument()
  })

  it('renders minimap when minimapVisible is true', () => {
    _minimapVisible.value = true
    render(<WorkflowCanvas />)
    expect(screen.getByTestId('minimap')).toBeInTheDocument()
  })

  it('wires selection change callback', () => {
    render(<WorkflowCanvas />)
    const rf = screen.getByTestId('react-flow')
    expect(rf.dataset.onSelectionChange).toBe('yes')
  })

  it('wires connect callback', () => {
    render(<WorkflowCanvas />)
    const rf = screen.getByTestId('react-flow')
    expect(rf.dataset.onConnect).toBe('yes')
  })

  it('wires delete callbacks', () => {
    render(<WorkflowCanvas />)
    const rf = screen.getByTestId('react-flow')
    expect(rf.dataset.onNodesDelete).toBe('yes')
    expect(rf.dataset.onEdgesDelete).toBe('yes')
  })

  it('wires reconnect callback', () => {
    render(<WorkflowCanvas />)
    const rf = screen.getByTestId('react-flow')
    expect(rf.dataset.onReconnect).toBe('yes')
  })
})

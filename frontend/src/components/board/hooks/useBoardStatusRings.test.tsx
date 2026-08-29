import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { ThemeProvider } from '@mui/material/styles'
import type { ReactNode } from 'react'
import { createAppTheme } from '@/theme'
import { boardStore, workflowLiveStore, workflowExecutionStore } from '@/stores'
import { useBoardStatusRings } from './useBoardStatusRings'
import { BOARD_RING } from '../constants'
import type { BaselineStepState, LiveDispatch } from '@/stores/workflowLiveStore'
import type { StepExecutionState, StepExecutionStatus } from '@/stores/workflowExecutionStore'

const theme = createAppTheme('midnight')
const palette = theme.palette.statusPalette

const wrapper = ({ children }: { children: ReactNode }) => (
  <ThemeProvider theme={theme}>{children}</ThemeProvider>
)

const ELEMENT_ID = 'el-1'
const STEP_ID = 'step-1'

const makeBaseline = (o: Partial<BaselineStepState> = {}): BaselineStepState => ({
  stepId: STEP_ID,
  name: 'Scanner',
  executionMode: 'workforce',
  baselineStatus: 'idle',
  pinned: false,
  hasRunSummary: false,
  isRunningInActiveRun: false,
  ...o,
})

const makeRunState = (status: StepExecutionStatus): StepExecutionState => ({
  status,
  stepName: 'Scanner',
  agentId: null,
  executionId: 'exec-1',
  output: null,
  error: null,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  startedAt: null,
  completedAt: null,
})

const makeDispatch = (status: string): LiveDispatch => ({
  stepId: STEP_ID,
  executionId: 'd1',
  status,
  instruction: 'Add a researcher',
  createdAt: '2025-01-01T00:00:00Z',
  result: null,
  traceLen: 0,
  source: 'registry',
})

const rings = (zoom = 1) =>
  renderHook(() => useBoardStatusRings(zoom), { wrapper }).result.current

beforeEach(() => {
  boardStore.store.setState({ elementStepMap: { [ELEMENT_ID]: STEP_ID } })
  workflowLiveStore.store.setState({ baselineByStep: {}, dispatches: [] })
  workflowExecutionStore.store.setState({ stepStates: {} })
})

describe('useBoardStatusRings', () => {
  it('finds the box under the element id the user drew it as', () => {
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('running') } })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.running })
  })

  it('keys by step id when the element map is empty', () => {
    boardStore.store.setState({ elementStepMap: {} })
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('running') } })
    expect(rings().get(STEP_ID)).toMatchObject({ color: palette.running })
  })

  /**
   * Regression: a node went blue while being designed, then dropped back to a
   * bare outline and stayed there until a refresh.
   *
   * `sync_canvas_elements` rebuilds the board from the steps whenever the
   * manager agent touches a node, and the boxes it writes are keyed by step id.
   * `refreshBoardElements` swaps them in but leaves `elementStepMap` holding the
   * client ids from the last submit, so a ring keyed only by the mapped id
   * landed on an element that was no longer on the board.
   */
  it('keys by step id too, so a board rebuilt from steps keeps its rings', () => {
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('running') } })
    const map = rings()
    expect(map.get(STEP_ID)).toMatchObject({ color: palette.running })
    expect(map.get(ELEMENT_ID)).toMatchObject({ color: palette.running })
  })

  it('survives a stale mapping left over from a deleted element', () => {
    boardStore.store.setState({ elementStepMap: { 'gone-from-the-board': STEP_ID } })
    workflowLiveStore.store.setState({
      baselineByStep: { [STEP_ID]: makeBaseline({ baselineStatus: 'configured' }) },
    })
    expect(rings().get(STEP_ID)).toMatchObject({ color: palette.designed })
  })

  // A box the user drew but never submitted matches no step, so stays bare.
  it('rings nothing when no step has any status', () => {
    boardStore.store.setState({ elementStepMap: {} })
    expect(rings().size).toBe(0)
  })

  it('omits an idle step rather than ringing it', () => {
    expect(rings().size).toBe(0)
  })

  /**
   * The layering that makes the board agree with the sidebar: a pinned step
   * reads as finished with no run overlay present at all.
   */
  it('rings a pinned step as finished with no active run', () => {
    workflowLiveStore.store.setState({
      baselineByStep: { [STEP_ID]: makeBaseline({ pinned: true }) },
    })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.finished })
  })

  it('lets the run overlay beat the baseline', () => {
    workflowLiveStore.store.setState({
      baselineByStep: { [STEP_ID]: makeBaseline({ baselineStatus: 'configured' }) },
    })
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('error') } })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.failed, glow: true })
  })

  // Agents being designed right now — the state the board exists to show.
  it('rings a step whose design is in flight as designing, and breathes', () => {
    workflowLiveStore.store.setState({ dispatches: [makeDispatch('running')] })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.designing, pulse: true })
  })

  it('rings a failed design attempt red, not blue', () => {
    workflowLiveStore.store.setState({ dispatches: [makeDispatch('failed')] })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.failed })
  })

  it('rings a configured-but-never-run step on the design axis', () => {
    workflowLiveStore.store.setState({
      baselineByStep: { [STEP_ID]: makeBaseline({ baselineStatus: 'configured' }) },
    })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.designed, pulse: false })
  })

  it('dims a skipped step without dashing it — dash means undesigned', () => {
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('skipped') } })
    expect(rings().get(ELEMENT_ID)).toMatchObject({ color: palette.skipped, dim: true })
  })

  it('stops breathing when zoomed out but keeps the ring', () => {
    workflowExecutionStore.store.setState({ stepStates: { [STEP_ID]: makeRunState('running') } })
    const zoomedOut = rings(BOARD_RING.ANIMATE_MIN_ZOOM - 0.1).get(ELEMENT_ID)
    expect(zoomedOut).toMatchObject({ color: palette.running, pulse: false })
  })
})

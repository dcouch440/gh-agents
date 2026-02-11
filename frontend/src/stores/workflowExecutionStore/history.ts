import { api } from '@/api'
import { extractError } from '../lib'
import { store, initialState } from './_store'

const fetchRuns = async (workflowId: string): Promise<void> => {
  store.setState({ historyLoading: true, historyError: null })
  try {
    const data = await api.workflows.listExecutions(workflowId)
    store.setState({ runs: data, historyLoading: false })
  } catch (e) {
    store.setState({ historyLoading: false, historyError: extractError('workflowExecution', e) })
  }
}

const viewHistoricalRun = (runId: string): void => {
  const { runs } = store.getState()
  // Single .find() on a small array, not inside a loop — acceptable
  const run = runs.find((r) => r.id === runId) ?? null
  store.setState({
    viewMode: 'history',
    selectedHistoricalRunId: runId,
    historicalRun: run,
  })
}

const returnToLive = (): void => {
  store.setState({
    viewMode: 'live',
    selectedHistoricalRunId: null,
    historicalRun: null,
  })
}

const reset = (): void => {
  store.setState({ ...initialState })
}

export { fetchRuns, viewHistoricalRun, returnToLive, reset }

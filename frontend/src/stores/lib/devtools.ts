// ============================================================================
// Devtools — Logger Middleware for Stores
// ============================================================================

import type { StoreApi } from './types'

type LoggerConfig = {
  enabled?: boolean
  collapsed?: boolean
  diff?: boolean
  timestamp?: boolean
  colors?: {
    title?: string
    prevState?: string
    action?: string
    nextState?: string
    error?: string
  }
}

const DEFAULT_CONFIG: Required<LoggerConfig> = {
  enabled: import.meta.env.DEV && !import.meta.env.VITEST,
  collapsed: false,
  diff: true,
  timestamp: true,
  colors: {
    title: '#3b82f6',
    prevState: '#6b7280',
    action: '#8b5cf6',
    nextState: '#10b981',
    error: '#ef4444',
  },
}

let globalConfig = { ...DEFAULT_CONFIG }

// ── Public API ───────────────────────────────────────────────────────────────

const configureLogger = (config: LoggerConfig): void => {
  globalConfig = { ...globalConfig, ...config }
}

const enableLogger = (): void => {
  globalConfig.enabled = true
}

const disableLogger = (): void => {
  globalConfig.enabled = false
}

// ── Logger Middleware ────────────────────────────────────────────────────────

const logger = <T>(name: string, store: StoreApi<T>): StoreApi<T> => {
  const original = store.setState

  store.setState = (partial) => {
    if (!globalConfig.enabled) {
      original(partial)
      return
    }

    const prevState = store.getState()
    const startTime = performance.now()

    // Execute the update
    original(partial)

    const nextState = store.getState()
    const duration = performance.now() - startTime

    // Infer action name from call stack (best effort)
    const actionName = inferActionName()

    // Log the state change
    logStateChange(name, actionName, prevState, nextState, duration)
  }

  return store
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const inferActionName = (): string => {
  const stack = new Error().stack ?? ''
  const lines = stack.split('\n')

  // Look for the first line that's not internal (createStore, setState, etc.)
  for (const line of lines) {
    if (
      line.includes('at ') &&
      !line.includes('setState') &&
      !line.includes('createStore') &&
      !line.includes('logger') &&
      !line.includes('devtools')
    ) {
      const match = line.match(/at (\w+)/)
      if (match) return match[1]
    }
  }

  return 'anonymous'
}

const logStateChange = <T>(
  storeName: string,
  actionName: string,
  prevState: T,
  nextState: T,
  duration: number,
): void => {
  const timestamp = globalConfig.timestamp ? new Date().toLocaleTimeString() : ''

  /* eslint-disable no-console */
  const logFn = globalConfig.collapsed ? console.groupCollapsed : console.group

  // Title with badge
  logFn(
    `%c ${storeName} %c ${actionName} %c${timestamp ? ` ${timestamp}` : ''}`,
    `background: ${globalConfig.colors.title}; color: white; padding: 2px 6px; border-radius: 3px 0 0 3px; font-weight: bold;`,
    `background: ${globalConfig.colors.action}; color: white; padding: 2px 6px; border-radius: 0 3px 3px 0; font-weight: bold;`,
    'color: #6b7280; font-size: 0.9em;',
  )

  // Previous state
  console.log(
    '%cprev state',
    `color: ${globalConfig.colors.prevState}; font-weight: bold;`,
    prevState,
  )

  // Diff (if enabled)
  if (globalConfig.diff) {
    const changes = computeDiff(prevState, nextState)
    if (changes.length > 0) {
      console.log('%cdiff', 'color: #f59e0b; font-weight: bold;')
      for (const change of changes) {
        console.log(
          `  %c${change.key}%c: %c${JSON.stringify(change.prev)}%c → %c${JSON.stringify(change.next)}`,
          'color: #8b5cf6; font-weight: bold;',
          'color: inherit;',
          'color: #ef4444;',
          'color: inherit;',
          'color: #10b981;',
        )
      }
    }
  }

  // Next state
  console.log(
    '%cnext state',
    `color: ${globalConfig.colors.nextState}; font-weight: bold;`,
    nextState,
  )

  // Performance
  console.log(
    `%cduration: ${duration.toFixed(2)}ms`,
    'color: #6b7280; font-size: 0.9em;',
  )

  console.groupEnd()
  /* eslint-enable no-console */
}

const computeDiff = <T>(prev: T, next: T): Array<{ key: string; prev: unknown; next: unknown }> => {
  if (typeof prev !== 'object' || typeof next !== 'object' || prev === null || next === null) {
    return []
  }

  const changes: Array<{ key: string; prev: unknown; next: unknown }> = []
  const allKeys = new Set([...Object.keys(prev), ...Object.keys(next)])

  for (const key of allKeys) {
    const prevVal = prev[key as keyof T]
    const nextVal = next[key as keyof T]

    if (!Object.is(prevVal, nextVal)) {
      changes.push({ key, prev: prevVal, next: nextVal })
    }
  }

  return changes
}

// ── Exports ──────────────────────────────────────────────────────────────────

export { logger, configureLogger, enableLogger, disableLogger }
export type { LoggerConfig }

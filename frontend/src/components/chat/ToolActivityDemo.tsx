import { useState, useEffect, useRef, useCallback } from 'react'
import { ToolActivityFeed } from './ToolActivityFeed'
import type { ToolEvent } from './ToolActivityFeed'
import type { ToolStatus } from './ToolActivityBox'

type ScriptStep = {
  toolName: string
  detail: string
  duration: number
  startDelay: number
}

const SCRIPT: ScriptStep[] = [
  { toolName: 'search_files', detail: 'searching src/auth/...', duration: 1200, startDelay: 0 },
  { toolName: 'read_file', detail: 'reading src/auth/mod.rs', duration: 800, startDelay: 600 },
  { toolName: 'analyze_code', detail: 'analyzing module structure...', duration: 2000, startDelay: 1600 },
  { toolName: 'write_file', detail: 'writing src/auth/middleware.rs', duration: 1500, startDelay: 3200 },
]

const HINTS = [
  'Atlas is coordinating task decomposition...',
  'Forge is writing src/auth/middleware.rs...',
  'Shield is reviewing the diff...',
  'Lens is scanning for vulnerabilities...',
  'Pixel is updating the frontend types...',
]

const ASSISTANT_TEXT = 'I found the auth module at src/auth/mod.rs. It contains the JWT middleware, session handling, and rate limiting logic. I\'ve updated the middleware to support the new token format.'

type DemoEvent = ToolEvent & { scriptIndex: number }

function ToolActivityDemo() {
  const [events, setEvents] = useState<DemoEvent[]>([])
  const [hint, setHint] = useState<string | null>(null)
  const [userMessage] = useState('Find the auth module and update the middleware')
  const [assistantText, setAssistantText] = useState('')
  const [showCursor, setShowCursor] = useState(false)
  const cycleRef = useRef(0)
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([])

  const clearTimers = useCallback(() => {
    for (const t of timersRef.current) clearTimeout(t)
    timersRef.current = []
  }, [])

  const runCycle = useCallback(() => {
    const cycle = cycleRef.current
    setEvents([])
    setAssistantText('')
    setShowCursor(false)
    setHint(null)
    clearTimers()

    let hintIndex = 0

    for (const [i, step] of SCRIPT.entries()) {
      const id = `${cycle}-${i}`

      // Start the tool
      const startTimer = setTimeout(() => {
        setEvents((prev) => [
          ...prev,
          {
            id,
            toolName: step.toolName,
            status: 'running' as ToolStatus,
            startedAt: Date.now(),
            completedAt: null,
            detail: step.detail,
            scriptIndex: i,
          },
        ])
        setHint(HINTS[hintIndex % HINTS.length] ?? null)
        hintIndex++
      }, step.startDelay)
      timersRef.current.push(startTimer)

      // Complete the tool
      const completeTimer = setTimeout(() => {
        setEvents((prev) =>
          prev.map((e) =>
            e.id === id
              ? { ...e, status: 'completed' as ToolStatus, completedAt: Date.now() }
              : e,
          ),
        )
        setHint(HINTS[hintIndex % HINTS.length] ?? null)
        hintIndex++
      }, step.startDelay + step.duration)
      timersRef.current.push(completeTimer)
    }

    // After all tools complete, stream the assistant response
    const lastStep = SCRIPT[SCRIPT.length - 1]
    if (!lastStep) return
    const responseStart = lastStep.startDelay + lastStep.duration + 300

    const cursorTimer = setTimeout(() => {
      setShowCursor(true)
      setHint(null)
    }, responseStart)
    timersRef.current.push(cursorTimer)

    // Type out the assistant text character by character
    for (let i = 0; i < ASSISTANT_TEXT.length; i++) {
      const charTimer = setTimeout(() => {
        setAssistantText(ASSISTANT_TEXT.slice(0, i + 1))
      }, responseStart + i * 18)
      timersRef.current.push(charTimer)
    }

    // Hide cursor after typing done
    const doneTimer = setTimeout(() => {
      setShowCursor(false)
    }, responseStart + ASSISTANT_TEXT.length * 18 + 200)
    timersRef.current.push(doneTimer)

    // Restart cycle
    const restartTimer = setTimeout(() => {
      cycleRef.current++
      runCycle()
    }, responseStart + ASSISTANT_TEXT.length * 18 + 2500)
    timersRef.current.push(restartTimer)
  }, [clearTimers])

  useEffect(() => {
    runCycle()
    return clearTimers
  }, [runCycle, clearTimers])

  return (
    <div className="chat-demo">
      <div className="chat-bubble chat-bubble--user">{userMessage}</div>

      {events.length > 0 ? (
        <ToolActivityFeed events={events} hint={hint} now={Date.now()} />
      ) : null}

      {assistantText.length > 0 ? (
        <div className="chat-bubble chat-bubble--assistant">
          {assistantText}
          {showCursor ? <span className="chat-bubble__cursor" /> : null}
        </div>
      ) : null}
    </div>
  )
}

export { ToolActivityDemo }

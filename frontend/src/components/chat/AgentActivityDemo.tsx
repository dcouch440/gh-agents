import { useState, useEffect, useRef, useCallback } from 'react'
import { AgentActivityModule } from './AgentActivityModule'
import type { Agent, ActivityLine } from './AgentActivityModule'

type ScriptStep = {
  agentName: string
  toolName: string
  summary: string
  duration: number
  startDelay: number
  statusText: string
}

const SCRIPT: ScriptStep[] = [
  {
    agentName: 'Atlas',
    toolName: 'search_files',
    summary: 'found 3 matches in src/auth',
    duration: 1200,
    startDelay: 0,
    statusText: 'Atlas is searching the codebase...',
  },
  {
    agentName: 'Forge',
    toolName: 'read_file',
    summary: 'loaded src/auth/mod.rs',
    duration: 800,
    startDelay: 600,
    statusText: 'Forge is reading src/auth/mod.rs...',
  },
  {
    agentName: 'Shield',
    toolName: 'analyze_code',
    summary: 'analyzed module structure',
    duration: 2000,
    startDelay: 1600,
    statusText: 'Shield is analyzing module structure...',
  },
  {
    agentName: 'Forge',
    toolName: 'write_file',
    summary: 'writing src/auth/middleware.rs',
    duration: 1500,
    startDelay: 3200,
    statusText: 'Forge is writing src/auth/middleware.rs...',
  },
]

const ASSISTANT_TEXT =
  "I found the auth module at src/auth/mod.rs. It contains the JWT middleware, session handling, and rate limiting logic. I've updated the middleware to support the new token format."

function AgentActivityDemo() {
  const [agents, setAgents] = useState<Agent[]>([])
  const [activities, setActivities] = useState<ActivityLine[]>([])
  const [statusText, setStatusText] = useState<string | null>(null)
  const [toolCallCount, setToolCallCount] = useState(0)
  const [userMessage] = useState('find the auth module and update the middleware')
  const [assistantText, setAssistantText] = useState('')
  const [showCursor, setShowCursor] = useState(false)
  const cycleRef = useRef(0)
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([])

  const clearTimers = useCallback(() => {
    for (const t of timersRef.current) clearTimeout(t)
    timersRef.current = []
  }, [])

  const addAgent = (name: string) => {
    setAgents((prev) => {
      if (prev.some((a) => a.name === name)) {
        return prev.map((a) => (a.name === name ? { ...a, status: 'active' as const } : a))
      }
      return [...prev, { id: name, name, status: 'active' as const }]
    })
  }

  const idleAgent = (name: string) => {
    setAgents((prev) => prev.map((a) => (a.name === name ? { ...a, status: 'idle' as const } : a)))
  }

  const runCycle = useCallback(() => {
    const cycle = cycleRef.current
    setAgents([])
    setActivities([])
    setStatusText(null)
    setToolCallCount(0)
    setAssistantText('')
    setShowCursor(false)
    clearTimers()

    for (const [i, step] of SCRIPT.entries()) {
      const id = `${cycle}-${i}`

      // Start tool
      const startTimer = setTimeout(() => {
        addAgent(step.agentName)
        setStatusText(step.statusText)
        setToolCallCount((c) => c + 1)
        setActivities((prev) => [
          ...prev,
          { id, toolName: step.toolName, status: 'running', summary: step.summary },
        ])
      }, step.startDelay)
      timersRef.current.push(startTimer)

      // Complete tool
      const completeTimer = setTimeout(() => {
        setActivities((prev) =>
          prev.map((a) => (a.id === id ? { ...a, status: 'completed' as const } : a)),
        )
        idleAgent(step.agentName)
      }, step.startDelay + step.duration)
      timersRef.current.push(completeTimer)
    }

    // After all tools complete
    const lastStep = SCRIPT[SCRIPT.length - 1]
    if (!lastStep) return
    const responseStart = lastStep.startDelay + lastStep.duration + 300

    const doneTimer = setTimeout(() => {
      setStatusText('Task complete')
      setAgents((prev) => prev.map((a) => ({ ...a, status: 'done' as const })))
      setShowCursor(true)
    }, responseStart)
    timersRef.current.push(doneTimer)

    // Type assistant text
    for (let i = 0; i < ASSISTANT_TEXT.length; i++) {
      const charTimer = setTimeout(() => {
        setAssistantText(ASSISTANT_TEXT.slice(0, i + 1))
      }, responseStart + i * 18)
      timersRef.current.push(charTimer)
    }

    // Hide cursor
    const cursorDone = setTimeout(() => {
      setShowCursor(false)
    }, responseStart + ASSISTANT_TEXT.length * 18 + 200)
    timersRef.current.push(cursorDone)

    // Restart
    const restart = setTimeout(() => {
      cycleRef.current++
      runCycle()
    }, responseStart + ASSISTANT_TEXT.length * 18 + 2500)
    timersRef.current.push(restart)
  }, [clearTimers])

  useEffect(() => {
    runCycle()
    return clearTimers
  }, [runCycle, clearTimers])

  return (
    <div className="activity-demo">
      <div className="activity-demo__prompt">&gt; {userMessage}</div>

      <AgentActivityModule
        agents={agents}
        statusText={statusText}
        activities={activities}
        toolCallCount={toolCallCount}
      />

      {assistantText.length > 0 ? (
        <div className="activity-demo__response">
          {assistantText}
          {showCursor ? <span className="activity-demo__cursor" /> : null}
        </div>
      ) : null}
    </div>
  )
}

export { AgentActivityDemo }

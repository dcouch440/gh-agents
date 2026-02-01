import { useContext } from 'react'
import { TaskContext } from '@/contexts/TaskContext'

const useTaskContext = () => {
  const ctx = useContext(TaskContext)
  if (!ctx) throw new Error('useTaskContext must be used within TaskProvider')
  return ctx
}

export { useTaskContext }

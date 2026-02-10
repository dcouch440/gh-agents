import { useContext } from 'react'
import { ThemeModeContext } from '@/contexts/ThemeModeContext'

const useThemeMode = () => {
  const ctx = useContext(ThemeModeContext)
  if (!ctx) throw new Error('useThemeMode must be used within ThemeModeProvider')
  return ctx
}

export { useThemeMode }

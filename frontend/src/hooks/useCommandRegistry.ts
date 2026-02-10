import { useContext, useEffect } from 'react'
import { CommandPaletteContext } from '@/contexts/CommandPaletteContext'
import type { CommandItem } from '@/contexts/CommandPaletteContext'

const useCommandRegistry = (commands: CommandItem[]) => {
  const ctx = useContext(CommandPaletteContext)
  if (!ctx) throw new Error('useCommandRegistry must be used within CommandPaletteProvider')

  const { registerCommands, unregisterCommands } = ctx

  useEffect(() => {
    if (commands.length === 0) return
    registerCommands(commands)
    const ids = commands.map((c) => c.id)
    return () => unregisterCommands(ids)
    // Only re-register when the command ids change
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [commands.map((c) => c.id).join(','), registerCommands, unregisterCommands])
}

export { useCommandRegistry }

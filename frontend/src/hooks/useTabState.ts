import { useState, useCallback } from 'react'

type TabState<T extends string = string> = {
  value: T
  onChange: (value: T) => void
}

const useTabState = <T extends string>(defaultValue: T): TabState<T> => {
  const [value, setValue] = useState<T>(defaultValue)
  const onChange = useCallback((next: T) => {
    setValue(next)
  }, [])
  return { value, onChange }
}

export { useTabState }
export type { TabState }

import { useCallback, useEffect, useRef, useState } from 'react'
import { InputBase, Box, IconButton } from '@mui/material'
import SearchOutlined from '@mui/icons-material/SearchOutlined'
import CloseOutlined from '@mui/icons-material/CloseOutlined'

type SearchInputProps = {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  debounceMs?: number
  autoFocus?: boolean
}

function SearchInput({ value, onChange, placeholder = 'Search...', debounceMs = 300, autoFocus }: SearchInputProps) {
  const [localValue, setLocalValue] = useState(value)
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(null)

  // Sync from parent
  useEffect(() => {
    setLocalValue(value)
  }, [value])

  const handleChange = useCallback(
    (newValue: string) => {
      setLocalValue(newValue)
      if (timeoutRef.current) clearTimeout(timeoutRef.current)
      timeoutRef.current = setTimeout(() => onChange(newValue), debounceMs)
    },
    [onChange, debounceMs],
  )

  const handleClear = useCallback(() => {
    setLocalValue('')
    onChange('')
  }, [onChange])

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current)
    }
  }, [])

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        border: 1,
        borderColor: 'divider',
        borderRadius: 1.5,
        px: 1.5,
        py: 0.5,
        '&:focus-within': {
          borderColor: 'primary.main',
        },
      }}
    >
      <SearchOutlined sx={{ color: 'text.secondary', fontSize: '1.1rem', mr: 1 }} />
      <InputBase
        value={localValue}
        onChange={(e) => handleChange(e.target.value)}
        placeholder={placeholder}
        fullWidth
        autoFocus={autoFocus}
        sx={{ fontSize: '0.875rem' }}
      />
      {localValue ? (
        <IconButton size="small" onClick={handleClear} sx={{ p: 0.25 }}>
          <CloseOutlined sx={{ fontSize: '0.9rem', color: 'text.secondary' }} />
        </IconButton>
      ) : null}
    </Box>
  )
}

export { SearchInput }
export type { SearchInputProps }

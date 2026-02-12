import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SettingsTab } from './SettingsTab'

describe('SettingsTab', () => {
  const onNameChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the name label', () => {
    render(<SettingsTab name="Write Docs" onNameChange={onNameChange} />)
    expect(screen.getByText('Name')).toBeInTheDocument()
  })

  it('renders current name as input value', () => {
    render(<SettingsTab name="Write Docs" onNameChange={onNameChange} />)
    expect(screen.getByDisplayValue('Write Docs')).toBeInTheDocument()
  })

  it('calls onNameChange when input value changes', () => {
    render(<SettingsTab name="Write Docs" onNameChange={onNameChange} />)
    fireEvent.change(screen.getByDisplayValue('Write Docs'), { target: { value: 'New Name' } })
    expect(onNameChange).toHaveBeenCalledWith('New Name')
  })

  it('shows placeholder when name is empty', () => {
    render(<SettingsTab name="" onNameChange={onNameChange} />)
    expect(screen.getByPlaceholderText('Documenter')).toBeInTheDocument()
  })
})

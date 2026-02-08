import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeProvider, createTheme } from '@mui/material'
import { PropertySelect } from './PropertySelect'
import type { PropertySelectOption } from './PropertySelect'

const theme = createTheme({ palette: { mode: 'dark' } })

const options: PropertySelectOption[] = [
  { value: 'a', label: 'Alpha', secondary: 'First' },
  { value: 'b', label: 'Beta', secondary: 'Second' },
  { value: 'c', label: 'Gamma' },
]

const renderSelect = (props: Partial<Parameters<typeof PropertySelect>[0]> = {}) => {
  const onChange = vi.fn()
  const result = render(
    <ThemeProvider theme={theme}>
      <PropertySelect
        value={null}
        options={options}
        onChange={onChange}
        {...props}
      />
    </ThemeProvider>,
  )
  return { onChange, ...result }
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('PropertySelect', () => {
  it('renders placeholder when value is null', () => {
    renderSelect({ placeholder: 'Pick one' })
    expect(screen.getByText('Pick one')).toBeInTheDocument()
  })

  it('renders selected option label', () => {
    renderSelect({ value: 'b' })
    expect(screen.getByText('Beta')).toBeInTheDocument()
  })

  it('calls onChange with option value on selection', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSelect()

    // Open the dropdown
    const selectButton = screen.getByRole('combobox')
    await user.click(selectButton)

    // Select an option from the dropdown listbox
    const listbox = within(screen.getByRole('listbox'))
    await user.click(listbox.getByText('Alpha'))

    expect(onChange).toHaveBeenCalledWith('a')
  })

  it('calls onChange with null when None is selected', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSelect({ value: 'a', allowNone: true })

    const selectButton = screen.getByRole('combobox')
    await user.click(selectButton)

    const listbox = within(screen.getByRole('listbox'))
    await user.click(listbox.getByText('None'))

    expect(onChange).toHaveBeenCalledWith(null)
  })

  it('renders all options in the dropdown', async () => {
    const user = userEvent.setup()
    renderSelect()

    const selectButton = screen.getByRole('combobox')
    await user.click(selectButton)

    const listbox = within(screen.getByRole('listbox'))
    expect(listbox.getByText('Alpha')).toBeInTheDocument()
    expect(listbox.getByText('Beta')).toBeInTheDocument()
    expect(listbox.getByText('Gamma')).toBeInTheDocument()
  })

  it('does not show None option when allowNone is false', async () => {
    const user = userEvent.setup()
    renderSelect({ allowNone: false })

    const selectButton = screen.getByRole('combobox')
    await user.click(selectButton)

    const listbox = within(screen.getByRole('listbox'))
    expect(listbox.queryByText('None')).not.toBeInTheDocument()
  })

  it('renders secondary text for options with secondary', async () => {
    const user = userEvent.setup()
    renderSelect()

    const selectButton = screen.getByRole('combobox')
    await user.click(selectButton)

    const listbox = within(screen.getByRole('listbox'))
    expect(listbox.getByText('First')).toBeInTheDocument()
    expect(listbox.getByText('Second')).toBeInTheDocument()
  })
})

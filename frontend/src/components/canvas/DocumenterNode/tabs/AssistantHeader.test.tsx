import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AssistantHeader } from './AssistantHeader'

describe('AssistantHeader', () => {
  it('renders label and clear button', () => {
    render(<AssistantHeader onClear={vi.fn()} disabled={false} />)
    expect(screen.getByText('Assistant')).toBeInTheDocument()
    expect(screen.getByRole('button')).toBeEnabled()
  })

  it('calls onClear when clear button clicked', async () => {
    const onClear = vi.fn()
    render(<AssistantHeader onClear={onClear} disabled={false} />)

    await userEvent.click(screen.getByRole('button'))
    expect(onClear).toHaveBeenCalledOnce()
  })

  it('disables clear button when disabled prop is true', () => {
    render(<AssistantHeader onClear={vi.fn()} disabled={true} />)
    expect(screen.getByRole('button')).toBeDisabled()
  })
})

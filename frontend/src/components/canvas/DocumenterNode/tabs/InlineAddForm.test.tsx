import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { InlineAddForm } from './InlineAddForm'

describe('InlineAddForm', () => {
  const onSubmit = vi.fn()
  const onCancel = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders all form fields', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    expect(screen.getByTestId('inline-add-name')).toBeInTheDocument()
    expect(screen.getByTestId('inline-add-description')).toBeInTheDocument()
    expect(screen.getByTestId('inline-add-target-length')).toBeInTheDocument()
  })

  it('renders Cancel and Add buttons', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    expect(screen.getByText('Cancel')).toBeInTheDocument()
    expect(screen.getByText('Add')).toBeInTheDocument()
  })

  it('disables Add button when name is empty', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    expect(screen.getByTestId('inline-add-submit')).toBeDisabled()
  })

  it('enables Add button when name has content', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    fireEvent.change(screen.getByTestId('inline-add-name'), { target: { value: 'My Doc' } })
    expect(screen.getByTestId('inline-add-submit')).not.toBeDisabled()
  })

  it('calls onSubmit with trimmed values on Add click', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    fireEvent.change(screen.getByTestId('inline-add-name'), { target: { value: '  My Doc  ' } })
    fireEvent.change(screen.getByTestId('inline-add-description'), { target: { value: '  A description  ' } })
    fireEvent.click(screen.getByTestId('inline-add-submit'))
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'My Doc',
      description: 'A description',
      target_length: 1000,
    })
  })

  it('submits with undefined description when empty', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    fireEvent.change(screen.getByTestId('inline-add-name'), { target: { value: 'Doc' } })
    fireEvent.click(screen.getByTestId('inline-add-submit'))
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'Doc',
      description: undefined,
      target_length: 1000,
    })
  })

  it('calls onCancel when Cancel is clicked', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    fireEvent.click(screen.getByText('Cancel'))
    expect(onCancel).toHaveBeenCalled()
  })

  it('submits on Enter when name is valid', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    const nameInput = screen.getByTestId('inline-add-name')
    fireEvent.change(nameInput, { target: { value: 'Doc' } })
    fireEvent.keyDown(nameInput, { key: 'Enter' })
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'Doc',
      description: undefined,
      target_length: 1000,
    })
  })

  it('does not submit on Enter when name is empty', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    const nameInput = screen.getByTestId('inline-add-name')
    fireEvent.keyDown(nameInput, { key: 'Enter' })
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('calls onCancel on Escape', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    const nameInput = screen.getByTestId('inline-add-name')
    fireEvent.keyDown(nameInput, { key: 'Escape' })
    expect(onCancel).toHaveBeenCalled()
  })

  it('does not submit when name is only whitespace', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    const nameInput = screen.getByTestId('inline-add-name')
    fireEvent.change(nameInput, { target: { value: '   ' } })
    fireEvent.keyDown(nameInput, { key: 'Enter' })
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('updates target length when valid number entered', () => {
    render(<InlineAddForm onSubmit={onSubmit} onCancel={onCancel} />)
    fireEvent.change(screen.getByTestId('inline-add-name'), { target: { value: 'Doc' } })
    fireEvent.change(screen.getByTestId('inline-add-target-length'), { target: { value: '500' } })
    fireEvent.click(screen.getByTestId('inline-add-submit'))
    expect(onSubmit).toHaveBeenCalledWith({
      name: 'Doc',
      description: undefined,
      target_length: 500,
    })
  })
})

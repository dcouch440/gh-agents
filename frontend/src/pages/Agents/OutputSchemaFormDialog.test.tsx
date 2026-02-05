import {describe, it, expect, vi, beforeEach} from 'vitest'
import {render, screen, waitFor} from '@testing-library/react'
import {userEvent} from '@testing-library/user-event'
import {OutputSchemaFormDialog} from './OutputSchemaFormDialog'
import {ApiError} from '@/api'

// Mock JsonEditor to avoid CodeMirror issues in tests
vi.mock('@/components/primitives', () => ({
  JsonEditor: ({value, onChange}: {value: string; onChange: (v: string) => void}) => (
    <textarea
      data-testid="json-editor"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      aria-label="JSON Schema"
    />
  ),
}))

const mockMutate = vi.hoisted(() => vi.fn())

const mockSchema = {
  id: 'mock-schema-id',
  name: 'Mock Schema',
  schema: {type: 'object', properties: {}},
  created_at: '2024-01-01T00:00:00Z',
}

vi.mock('@/hooks/useOutputSchemaMutations', () => ({
  useCreateOutputSchema: () => ({
    mutate: mockMutate,
    loading: false,
    error: null,
  }),
}))

// Setup default mock behavior to prevent undefined errors
mockMutate.mockResolvedValue(mockSchema)

describe('OutputSchemaFormDialog', () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onSave: vi.fn(),
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders when open', () => {
    render(<OutputSchemaFormDialog {...defaultProps} />)

    expect(screen.getByText('Create Output Schema')).toBeDefined()
    expect(screen.getByLabelText(/name/i)).toBeDefined()
    expect(screen.getByLabelText(/json schema/i)).toBeDefined()
  })

  it('does not render when closed', () => {
    render(<OutputSchemaFormDialog {...defaultProps} open={false} />)

    expect(screen.queryByText('Create Output Schema')).toBeNull()
  })

  it('validates required name field', async () => {
    const user = userEvent.setup()
    render(<OutputSchemaFormDialog {...defaultProps} />)

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(screen.getByText('Name is required')).toBeDefined()
    })

    expect(mockMutate).not.toHaveBeenCalled()
  })

  it('prevents typing more than 200 characters', () => {
    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)

    // The maxLength attribute prevents typing more than 200 chars
    expect((nameInput as HTMLInputElement).maxLength).toBe(200)
  })

  it('shows character count', async () => {
    const user = userEvent.setup()
    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test')

    await waitFor(() => {
      expect(screen.getByText('4/200 characters')).toBeDefined()
    })
  })

  it('validates JSON schema format', async () => {
    const user = userEvent.setup()
    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test Schema')

    // Change JSON to invalid syntax - use paste to avoid userEvent parsing issues
    const jsonEditor = screen.getByLabelText(/json schema/i)
    await user.clear(jsonEditor)
    await user.click(jsonEditor)
    await user.paste('not valid json')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(screen.getByText(/invalid json/i)).toBeDefined()
    })

    expect(mockMutate).not.toHaveBeenCalled()
  })

  it('calls onSave with new schema ID on success', async () => {
    const user = userEvent.setup()
    const mockSchema = {
      id: 'new-schema-id',
      name: 'Test Schema',
      schema: {type: 'object', properties: {}},
      created_at: '2024-01-01T00:00:00Z',
    }

    mockMutate.mockResolvedValue(mockSchema)

    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test Schema')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(mockMutate).toHaveBeenCalledWith({
        name: 'Test Schema',
        schema: {type: 'object', properties: {}},
      })
    })

    expect(defaultProps.onSave).toHaveBeenCalledWith('new-schema-id')
    expect(defaultProps.onClose).toHaveBeenCalled()
  })

  it('handles 409 conflict errors', async () => {
    const user = userEvent.setup()
    const conflictError = new ApiError(
      'http_error',
      'Conflict: name already exists',
      '/api/output-schemas',
      {status: 409, statusText: 'Conflict'}
    )

    // Override the default mock behavior for this test
    mockMutate.mockRejectedValueOnce(conflictError)

    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Duplicate Schema')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(screen.getByText('A schema with this name already exists')).toBeDefined()
    })

    expect(defaultProps.onSave).not.toHaveBeenCalled()
    expect(defaultProps.onClose).not.toHaveBeenCalled()
  })

  it('handles generic API errors', async () => {
    const user = userEvent.setup()
    const genericError = new Error('Network error')

    mockMutate.mockRejectedValue(genericError)

    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test Schema')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeDefined()
    })

    expect(defaultProps.onSave).not.toHaveBeenCalled()
  })

  it('closes and resets form on cancel', async () => {
    const user = userEvent.setup()
    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test Schema')

    const cancelButton = screen.getByRole('button', {name: /cancel/i})
    await user.click(cancelButton)

    expect(defaultProps.onClose).toHaveBeenCalled()
  })

  it('resets form after successful creation', async () => {
    const user = userEvent.setup()
    const mockSchema = {
      id: 'new-schema-id',
      name: 'Test Schema',
      description: '',
      schema: {type: 'object', properties: {}},
      user_id: 'user-1',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    }

    mockMutate.mockResolvedValue(mockSchema)

    const {rerender} = render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'Test Schema')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(defaultProps.onSave).toHaveBeenCalled()
    })

    rerender(<OutputSchemaFormDialog {...defaultProps} open={true} />)

    const nameInputAfter = screen.getByLabelText(/name/i)
    expect((nameInputAfter as HTMLInputElement).value).toBe('')
  })

  it('trims whitespace from name', async () => {
    const user = userEvent.setup()
    const mockSchema = {
      id: 'new-schema-id',
      name: 'Test Schema',
      schema: {type: 'object', properties: {}},
      created_at: '2024-01-01T00:00:00Z',
    }

    mockMutate.mockResolvedValue(mockSchema)

    render(<OutputSchemaFormDialog {...defaultProps} />)

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, '  Test Schema  ')

    const createButton = screen.getByRole('button', {name: /create/i})
    await user.click(createButton)

    await waitFor(() => {
      expect(mockMutate).toHaveBeenCalledWith({
        name: 'Test Schema',
        schema: {type: 'object', properties: {}},
      })
    })
  })
})

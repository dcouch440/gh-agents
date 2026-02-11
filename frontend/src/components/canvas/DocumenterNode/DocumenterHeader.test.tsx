import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { DocumenterHeader } from './DocumenterHeader'

describe('DocumenterHeader', () => {
  it('renders the step name', () => {
    render(<DocumenterHeader name="Write Docs" documentNames={[]} />)
    expect(screen.getByText('Write Docs')).toBeInTheDocument()
  })

  it('renders "No documents" when documentNames is empty', () => {
    render(<DocumenterHeader name="Writer" documentNames={[]} />)
    expect(screen.getByText('No documents')).toBeInTheDocument()
  })

  it('renders document names joined by middle dot', () => {
    render(<DocumenterHeader name="Writer" documentNames={['README', 'CHANGELOG']} />)
    expect(screen.getByText('README \u00b7 CHANGELOG')).toBeInTheDocument()
  })

  it('renders a single document name without separator', () => {
    render(<DocumenterHeader name="Writer" documentNames={['README']} />)
    expect(screen.getByText('README')).toBeInTheDocument()
  })

  it('renders the Protocol badge', () => {
    render(<DocumenterHeader name="Writer" documentNames={[]} />)
    expect(screen.getByText('Protocol')).toBeInTheDocument()
  })
})

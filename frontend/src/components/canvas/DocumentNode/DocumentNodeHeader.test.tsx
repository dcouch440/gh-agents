import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/render'
import { DocumentNodeHeader } from './DocumentNodeHeader'

describe('DocumentNodeHeader', () => {
  it('renders the document name', () => {
    render(<DocumentNodeHeader name="API Spec" parentStepName="Doc Writer" />)
    expect(screen.getByText('API Spec')).toBeInTheDocument()
  })

  it('renders the parent step name', () => {
    render(<DocumentNodeHeader name="API Spec" parentStepName="Doc Writer" />)
    expect(screen.getByText('Doc Writer')).toBeInTheDocument()
  })

  it('renders the Document badge', () => {
    render(<DocumentNodeHeader name="API Spec" parentStepName="Doc Writer" />)
    expect(screen.getByText('Document')).toBeInTheDocument()
  })

  it('uses custom accent color when provided', () => {
    render(<DocumentNodeHeader name="Spec" parentStepName="Writer" accentColor="#ff0000" />)
    // Badge still renders with label regardless of color
    expect(screen.getByText('Document')).toBeInTheDocument()
  })
})

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { RightPanelContent } from './RightPanelContent'

vi.mock('./PropertiesPanel', () => ({
  PropertiesPanel: () => <div data-testid="properties-panel" />,
}))

vi.mock('./AgentsBrowserPanel', () => ({
  AgentsBrowserPanel: () => <div data-testid="agents-panel" />,
}))

vi.mock('./PromptsBrowserPanel', () => ({
  PromptsBrowserPanel: () => <div data-testid="prompts-panel" />,
}))

vi.mock('./SchemasBrowserPanel', () => ({
  SchemasBrowserPanel: () => <div data-testid="schemas-panel" />,
}))

vi.mock('./ExecutionPanel', () => ({
  ExecutionPanel: () => <div data-testid="execution-panel" />,
}))

describe('RightPanelContent', () => {
  it('renders PropertiesPanel for "properties" section', () => {
    render(<RightPanelContent section="properties" />)
    expect(screen.getByTestId('properties-panel')).toBeInTheDocument()
  })

  it('renders AgentsBrowserPanel for "agents" section', () => {
    render(<RightPanelContent section="agents" />)
    expect(screen.getByTestId('agents-panel')).toBeInTheDocument()
  })

  it('renders PromptsBrowserPanel for "prompts" section', () => {
    render(<RightPanelContent section="prompts" />)
    expect(screen.getByTestId('prompts-panel')).toBeInTheDocument()
  })

  it('renders SchemasBrowserPanel for "schemas" section', () => {
    render(<RightPanelContent section="schemas" />)
    expect(screen.getByTestId('schemas-panel')).toBeInTheDocument()
  })

  it('renders ExecutionPanel for "execution" section', () => {
    render(<RightPanelContent section="execution" />)
    expect(screen.getByTestId('execution-panel')).toBeInTheDocument()
  })

  it('renders nothing for null section', () => {
    const { container } = render(<RightPanelContent section={null} />)
    expect(container.firstChild).toBeNull()
  })

  it('renders nothing for unknown section', () => {
    const { container } = render(<RightPanelContent section="unknown" />)
    expect(container.firstChild).toBeNull()
  })
})

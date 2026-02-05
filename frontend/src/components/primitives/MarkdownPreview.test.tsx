import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MarkdownPreview } from './MarkdownPreview'

describe('MarkdownPreview', () => {
  it('renders plain text', () => {
    render(<MarkdownPreview content="Hello world" />)
    expect(screen.getByText('Hello world')).toBeInTheDocument()
  })

  it('renders markdown bold', () => {
    render(<MarkdownPreview content="This is **bold** text" />)
    expect(screen.getByText('bold').tagName).toBe('STRONG')
  })

  it('strips thinking tags', () => {
    render(<MarkdownPreview content="<thinking>secret</thinking>visible" />)
    expect(screen.getByText('visible')).toBeInTheDocument()
    expect(screen.queryByText('secret')).not.toBeInTheDocument()
  })

  it('applies className', () => {
    const { container } = render(<MarkdownPreview content="test" className="custom" />)
    expect(container.firstChild).toHaveClass('custom')
  })

  it('renders code blocks inside pre elements', () => {
    const md = '```js\nconsole.log("hi")\n```'
    const { container } = render(<MarkdownPreview content={md} />)
    const preElement = container.querySelector('pre')
    expect(preElement).toBeInTheDocument()
    const codeElement = preElement?.querySelector('code')
    expect(codeElement).toBeInTheDocument()
  })
})

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CodeEditor } from './CodeEditor'

describe('CodeEditor', () => {
  it('renders a container element', () => {
    const { container } = render(
      <CodeEditor value="" onChange={() => {}} />
    )
    const editor = container.querySelector('.code-editor')
    expect(editor).toBeInTheDocument()
  })

  it('applies custom className', () => {
    const { container } = render(
      <CodeEditor value="" onChange={() => {}} className="custom" />
    )
    const editor = container.querySelector('.code-editor')
    expect(editor).toHaveClass('custom')
  })

  it('applies custom height', () => {
    const { container } = render(
      <CodeEditor value="" onChange={() => {}} height="500px" />
    )
    const editor = container.querySelector('.code-editor') as HTMLElement
    expect(editor.style.height).toBe('500px')
  })

  it('initializes CodeMirror editor inside container', () => {
    const { container } = render(
      <CodeEditor value="hello world" onChange={() => {}} />
    )
    const cmEditor = container.querySelector('.cm-editor')
    expect(cmEditor).toBeInTheDocument()
  })

  it('exposes EditorView via editorViewRef', () => {
    let capturedView: unknown = null
    render(
      <CodeEditor
        value="test"
        onChange={() => {}}
        editorViewRef={(v) => { capturedView = v }}
      />
    )
    expect(capturedView).not.toBeNull()
  })

  it('displays initial value in the editor', () => {
    const { container } = render(
      <CodeEditor value="# Hello" onChange={() => {}} />
    )
    expect(container.textContent).toContain('# Hello')
  })

  it('renders placeholder when provided and value is empty', () => {
    render(
      <CodeEditor value="" onChange={() => {}} placeholder="Type here..." />
    )
    expect(screen.getByText('Type here...')).toBeInTheDocument()
  })
})

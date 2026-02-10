import { useEffect, useRef, useCallback } from 'react'
import { Box } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import { EditorView, placeholder as cmPlaceholder, keymap } from '@codemirror/view'
import { EditorState, type Extension } from '@codemirror/state'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { getEditorThemeExtensions } from './editorThemes'
import { linter } from '@codemirror/lint'

type JsonEditorProps = {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  readOnly?: boolean
  height?: string
  className?: string
}

function JsonEditor({ value, onChange, placeholder, readOnly = false, height = '300px', className }: JsonEditorProps) {
  const theme = useTheme()
  const mode = theme.palette.mode
  const containerRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const isUpdatingRef = useRef(false)
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange

  const getLanguageExtension = useCallback((): Extension[] => {
    return [json(), linter(jsonParseLinter())]
  }, [])

  useEffect(() => {
    if (!containerRef.current) return

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !isUpdatingRef.current) {
        onChangeRef.current(update.state.doc.toString())
      }
    })

    const baseExtensions: Extension[] = [
      keymap.of([...defaultKeymap, ...historyKeymap]),
      history(),
      ...getEditorThemeExtensions(mode),
      EditorView.lineWrapping,
      updateListener,
      ...getLanguageExtension(),
    ]

    if (placeholder) {
      baseExtensions.push(cmPlaceholder(placeholder))
    }

    if (readOnly) {
      baseExtensions.push(EditorState.readOnly.of(true))
    }

    const state = EditorState.create({
      doc: value,
      extensions: baseExtensions,
    })

    const view = new EditorView({
      state,
      parent: containerRef.current,
    })

    viewRef.current = view

    return () => {
      view.destroy()
      viewRef.current = null
    }
    // Rebuild when theme mode changes; other deps intentionally omitted
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode])

  // Sync external value changes
  useEffect(() => {
    const view = viewRef.current
    if (!view) return

    const currentDoc = view.state.doc.toString()
    if (currentDoc === value) return

    isUpdatingRef.current = true
    view.dispatch({
      changes: { from: 0, to: currentDoc.length, insert: value },
    })
    isUpdatingRef.current = false
  }, [value])

  return (
    <Box
      ref={containerRef}
      className={className}
      sx={{
        height,
        border: 1,
        borderColor: 'divider',
        borderRadius: 1,
        overflow: 'hidden',
        fontFamily: 'monospace',
        '&:focus-within': {
          borderColor: 'primary.main',
        },
        '& .cm-editor': {
          height: '100%',
        },
        '& .cm-scroller': {
          overflow: 'auto',
        },
      }}
    />
  )
}

export { JsonEditor }
export type { JsonEditorProps }

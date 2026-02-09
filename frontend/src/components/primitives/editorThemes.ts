import { EditorView } from '@codemirror/view'
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { tags } from '@lezer/highlight'
import { oneDarkTheme, oneDarkHighlightStyle } from '@codemirror/theme-one-dark'
import type { Extension } from '@codemirror/state'

/* ── Light earthy theme ─────────────────────────────────── */

const lightEditorTheme = EditorView.theme(
  {
    '&': {
      backgroundColor: '#f5f0e8',
      color: '#3d2b1f',
    },
    '.cm-content': {
      caretColor: '#c0502e',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: '#c0502e',
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection':
      {
        backgroundColor: 'rgba(192, 80, 46, 0.12)',
      },
    '.cm-panels': {
      backgroundColor: '#f0ebe3',
      color: '#3d2b1f',
    },
    '.cm-panels.cm-panels-top': {
      borderBottom: '1px solid rgba(61, 43, 31, 0.08)',
    },
    '.cm-panels.cm-panels-bottom': {
      borderTop: '1px solid rgba(61, 43, 31, 0.08)',
    },
    '.cm-searchMatch': {
      backgroundColor: 'rgba(192, 80, 46, 0.15)',
      outline: '1px solid rgba(192, 80, 46, 0.3)',
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: 'rgba(107, 143, 113, 0.2)',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(61, 43, 31, 0.03)',
    },
    '.cm-selectionMatch': {
      backgroundColor: 'rgba(192, 80, 46, 0.08)',
    },
    '&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket': {
      backgroundColor: 'rgba(107, 143, 113, 0.25)',
    },
    '.cm-gutters': {
      backgroundColor: '#f0ebe3',
      color: '#a89b8c',
      borderRight: '1px solid rgba(61, 43, 31, 0.06)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(61, 43, 31, 0.04)',
    },
    '.cm-foldPlaceholder': {
      backgroundColor: 'rgba(61, 43, 31, 0.05)',
      border: 'none',
      color: '#7a6858',
    },
    '.cm-tooltip': {
      backgroundColor: '#faf7f2',
      border: '1px solid rgba(61, 43, 31, 0.1)',
    },
    '.cm-tooltip .cm-tooltip-arrow:before': {
      borderTopColor: 'rgba(61, 43, 31, 0.1)',
      borderBottomColor: 'rgba(61, 43, 31, 0.1)',
    },
    '.cm-tooltip .cm-tooltip-arrow:after': {
      borderTopColor: '#faf7f2',
      borderBottomColor: '#faf7f2',
    },
    '.cm-tooltip-autocomplete': {
      '& > ul > li[aria-selected]': {
        backgroundColor: 'rgba(192, 80, 46, 0.1)',
        color: '#3d2b1f',
      },
    },
    '.cm-placeholder': {
      color: '#a89b8c',
    },
  },
  { dark: false },
)

const lightHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: '#993e24' },
  { tag: [tags.name, tags.deleted, tags.character, tags.propertyName, tags.macroName], color: '#c0502e' },
  { tag: [tags.function(tags.variableName), tags.labelName], color: '#993e24' },
  { tag: [tags.color, tags.constant(tags.name), tags.standard(tags.name)], color: '#8b664c' },
  { tag: [tags.definition(tags.name), tags.separator], color: '#3d2b1f' },
  { tag: [tags.typeName, tags.className, tags.number, tags.changed, tags.annotation, tags.modifier, tags.self, tags.namespace], color: '#a06824' },
  { tag: [tags.operator, tags.operatorKeyword, tags.url, tags.escape, tags.regexp, tags.link, tags.special(tags.string)], color: '#8b664c' },
  { tag: [tags.meta, tags.comment], color: '#a89b8c' },
  { tag: tags.strong, fontWeight: 'bold' },
  { tag: tags.emphasis, fontStyle: 'italic' },
  { tag: tags.strikethrough, textDecoration: 'line-through' },
  { tag: tags.link, color: '#587a5e', textDecoration: 'underline' },
  { tag: tags.heading, fontWeight: 'bold', color: '#993e24' },
  { tag: [tags.atom, tags.bool, tags.special(tags.variableName)], color: '#a06824' },
  { tag: [tags.processingInstruction, tags.string, tags.inserted], color: '#587a5e' },
  { tag: tags.invalid, color: '#b5382a' },
])

/* ── Public API ──────────────────────────────────────────── */

const getEditorThemeExtensions = (mode: 'light' | 'dark'): Extension[] => {
  if (mode === 'light') {
    return [lightEditorTheme, syntaxHighlighting(lightHighlightStyle)]
  }
  return [oneDarkTheme, syntaxHighlighting(oneDarkHighlightStyle)]
}

export { getEditorThemeExtensions }

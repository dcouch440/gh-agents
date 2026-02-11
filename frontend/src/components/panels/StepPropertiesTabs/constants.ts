const EDITOR_CONTAINER_SX = {
  flex: 1,
  borderTop: 1,
  borderColor: 'divider',
  minHeight: 0,
  '& > div': { border: 'none', borderRadius: 0, height: '100%' },
  '& .cm-editor': { height: '100%' },
  '& .cm-scroller': { overflow: 'auto' },
  '& .cm-gutters': {
    backgroundColor: 'transparent',
    border: 'none',
  },
  '& .cm-lineNumbers .cm-gutterElement': {
    paddingLeft: '2px',
    paddingRight: '2px',
    minWidth: 'unset',
    fontSize: 10,
    opacity: 0.35,
  },
  '& .cm-content': { paddingLeft: '1px' },
} as const

const MUTED_EDITOR_CONTAINER_SX = {
  ...EDITOR_CONTAINER_SX,
  opacity: 0.5,
  backgroundColor: 'rgba(255,255,255,0.01)',
} as const

const SECTION_LABEL_SX = {
  fontSize: 9,
  fontWeight: 600,
  letterSpacing: '0.06em',
  textTransform: 'uppercase',
  color: 'text.disabled',
  px: '16px',
  pt: '10px',
  pb: '4px',
} as const

const SCHEMA_PREVIEW_SX = {
  mx: '16px',
  mb: '12px',
  p: '10px',
  borderRadius: '6px',
  backgroundColor: 'rgba(255,255,255,0.02)',
  border: 1,
  borderColor: 'divider',
  fontSize: 11,
  fontFamily: 'monospace',
  color: 'text.secondary',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  overflow: 'auto',
  maxHeight: 300,
} as const

export { EDITOR_CONTAINER_SX, MUTED_EDITOR_CONTAINER_SX, SECTION_LABEL_SX, SCHEMA_PREVIEW_SX }

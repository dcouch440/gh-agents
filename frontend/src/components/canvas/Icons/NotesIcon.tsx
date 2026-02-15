type NotesIconProps = {
  color?: string
  size?: number
}

function NotesIcon({ color = '#f85149', size = 24 }: NotesIconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M4 4a2 2 0 012-2h12a2 2 0 012 2v16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"
        fill={`${color}18`}
        stroke={color}
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      <line x1="8" y1="8" x2="16" y2="8" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.5" />
      <line x1="8" y1="12" x2="16" y2="12" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.5" />
      <line x1="8" y1="16" x2="12" y2="16" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.35" />
    </svg>
  )
}

export { NotesIcon }
export type { NotesIconProps }

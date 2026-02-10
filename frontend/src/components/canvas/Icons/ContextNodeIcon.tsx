type ContextNodeIconProps = {
  color?: string
  size?: number
}

function ContextNodeIcon({ color = '#10b981', size = 24 }: ContextNodeIconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M6 2h8l6 6v12a2 2 0 01-2 2H6a2 2 0 01-2-2V4a2 2 0 012-2z"
        fill={`${color}18`}
        stroke={color}
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      <path d="M14 2v6h6" stroke={color} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      <line x1="8" y1="13" x2="16" y2="13" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.5" />
      <line x1="8" y1="16" x2="13" y2="16" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.35" />
    </svg>
  )
}

export { ContextNodeIcon }
export type { ContextNodeIconProps }

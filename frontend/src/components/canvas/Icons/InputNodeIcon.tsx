type InputNodeIconProps = {
  color?: string
  size?: number
}

function InputNodeIcon({ color = '#f59e0b', size = 24 }: InputNodeIconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Pencil body */}
      <path
        d="M16.5 3.5l4 4L7 21H3v-4L16.5 3.5z"
        fill={`${color}18`}
        stroke={color}
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      {/* Pencil tip separation line */}
      <path d="M5 19l2-2" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.5" />
      {/* Edit line accent */}
      <path d="M14.5 5.5l4 4" stroke={color} strokeWidth="1.2" strokeLinecap="round" opacity="0.5" />
    </svg>
  )
}

export { InputNodeIcon }
export type { InputNodeIconProps }

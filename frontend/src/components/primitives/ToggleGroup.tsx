export type ToggleOption = {
  value: string
  label: string
}

export type ToggleGroupProps = {
  options: ToggleOption[]
  value: string
  onChange: (value: string) => void
  className?: string
}

export function ToggleGroup({ options, value, onChange, className }: ToggleGroupProps) {
  const groupClassName = ['toggle-group', className].filter(Boolean).join(' ')

  return (
    <div className={groupClassName}>
      {options.map((option) => {
        const btnClassName = option.value === value
          ? 'toggle-group__btn toggle-group__btn--active'
          : 'toggle-group__btn'

        return (
          <button
            key={option.value}
            type="button"
            className={btnClassName}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        )
      })}
    </div>
  )
}

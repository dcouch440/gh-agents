import { Checkbox, FormControlLabel } from '@mui/material'
import { InlineMarkdown } from './InlineMarkdown'

type PanelCheckboxProps = {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}

function PanelCheckbox({ label, checked, onChange }: PanelCheckboxProps) {
  return (
    <FormControlLabel
      control={
        <Checkbox
          checked={checked}
          onChange={(_, val) => onChange(val)}
          size="small"
          sx={{ py: 0.25 }}
        />
      }
      label={<InlineMarkdown content={label} />}
      sx={{
        mx: 0,
        '& .MuiFormControlLabel-label': {
          fontSize: '0.875rem',
        },
      }}
    />
  )
}

export { PanelCheckbox }
export type { PanelCheckboxProps }

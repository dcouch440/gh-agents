import Typography from '@mui/material/Typography'

type SectionLabelProps = {
  label: string
}

function SectionLabel({ label }: SectionLabelProps) {
  return (
    <Typography
      sx={{
        fontSize: 8,
        fontWeight: 600,
        textTransform: 'uppercase',
        color: 'text.disabled',
        letterSpacing: '0.06em',
        lineHeight: 1,
        mb: 0.5,
      }}
    >
      {label}
    </Typography>
  )
}

export { SectionLabel }
export type { SectionLabelProps }

import { Box, Typography } from '@mui/material'
import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'

function SettingsPage() {
  return (
    <FadeIn>
      <Box>
        <PageHeader
          title="Settings"
          description="Configure your application"
          breadcrumbs={[{ label: 'Home', path: '/' }, { label: 'Settings' }]}
        />

        <Box>
          <Typography variant="body1" color="text.secondary">
            General settings and configuration options will appear here.
          </Typography>
        </Box>
      </Box>
    </FadeIn>
  )
}

export { SettingsPage }

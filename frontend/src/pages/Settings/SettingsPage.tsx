import { useState } from 'react'
import { Box, Tabs, Tab, Typography } from '@mui/material'
import { FadeIn } from '@/components/animation'
import { PageHeader } from '@/components/primitives'
import { RouterModesTab } from './RouterModesTab'

type SettingsTab = 'overview' | 'router-modes'

function SettingsPage() {
  const [tab, setTab] = useState<SettingsTab>('overview')

  return (
    <FadeIn>
      <Box>
        <PageHeader
          title="Settings"
          description="Configure your application"
          breadcrumbs={[{ label: 'Home', path: '/' }, { label: 'Settings' }]}
        />

        <Box sx={{ borderBottom: 1, borderColor: 'divider', mb: 3 }}>
          <Tabs value={tab} onChange={(_, newValue) => setTab(newValue as SettingsTab)}>
            <Tab label="Overview" value="overview" />
            <Tab label="Router Modes" value="router-modes" />
          </Tabs>
        </Box>

        {tab === 'overview' && (
          <Box>
            <Typography variant="body1" color="text.secondary">
              General settings and configuration options will appear here.
            </Typography>
          </Box>
        )}

        {tab === 'router-modes' && <RouterModesTab />}
      </Box>
    </FadeIn>
  )
}

export { SettingsPage }

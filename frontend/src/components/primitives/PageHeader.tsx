import type { ReactNode } from 'react'
import { Box, Typography, Breadcrumbs as MuiBreadcrumbs, Link } from '@mui/material'
import { Link as RouterLink } from 'react-router-dom'

type BreadcrumbItem = {
  label: string
  path?: string
}

type PageHeaderProps = {
  title: string
  description?: string
  breadcrumbs?: BreadcrumbItem[]
  children?: ReactNode
}

function PageHeader({ title, description, breadcrumbs, children }: PageHeaderProps) {
  return (
    <Box sx={{ mb: 3 }}>
      {breadcrumbs && breadcrumbs.length > 0 ? (
        <MuiBreadcrumbs sx={{ mb: 1, '& .MuiBreadcrumbs-separator': { fontSize: '0.75rem' } }}>
          {breadcrumbs.map((crumb, i) =>
            crumb.path ? (
              <Link key={i} component={RouterLink} to={crumb.path} underline="hover" color="text.secondary" sx={{ fontSize: '0.8125rem' }}>
                {crumb.label}
              </Link>
            ) : (
              <Typography key={i} color="text.primary" sx={{ fontSize: '0.8125rem' }}>
                {crumb.label}
              </Typography>
            ),
          )}
        </MuiBreadcrumbs>
      ) : null}

      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <Box>
          <Typography variant="h4" component="h1" sx={{ fontWeight: 600 }}>
            {title}
          </Typography>
          {description ? (
            <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
              {description}
            </Typography>
          ) : null}
        </Box>
        {children ? <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>{children}</Box> : null}
      </Box>
    </Box>
  )
}

export { PageHeader }
export type { PageHeaderProps, BreadcrumbItem }

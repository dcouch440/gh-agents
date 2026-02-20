/* eslint-disable react-refresh/only-export-components */
import { type ReactElement } from 'react'
import { render, type RenderOptions } from '@testing-library/react'
import { ThemeProvider } from '@mui/material/styles'
import { createAppTheme } from '@/theme'

const testTheme = createAppTheme('midnight')

function TestProviders({ children }: { children: React.ReactNode }) {
  return <ThemeProvider theme={testTheme}>{children}</ThemeProvider>
}

const renderWithTheme = (ui: ReactElement, options?: Omit<RenderOptions, 'wrapper'>) => render(ui, { wrapper: TestProviders, ...options })

export * from '@testing-library/react'
export { renderWithTheme as render }

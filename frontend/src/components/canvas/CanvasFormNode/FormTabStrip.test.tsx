import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { FormTabStrip } from './FormTabStrip'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'
import ChatOutlined from '@mui/icons-material/ChatOutlined'
import type { CanvasFormTab } from './types'

const makeTabs = (): CanvasFormTab[] => [
  { id: 'chat', icon: ChatOutlined, tooltip: 'Chat', content: null },
  { id: 'settings', icon: SettingsOutlined, tooltip: 'Settings', content: null },
]

describe('FormTabStrip', () => {
  it('renders all tab icons', () => {
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={() => {}} accentColor="#3b82f6" />)
    expect(screen.getByTestId('tab-chat')).toBeInTheDocument()
    expect(screen.getByTestId('tab-settings')).toBeInTheDocument()
  })

  it('marks active tab with aria-selected', () => {
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={() => {}} accentColor="#3b82f6" />)
    expect(screen.getByTestId('tab-chat')).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByTestId('tab-settings')).toHaveAttribute('aria-selected', 'false')
  })

  it('calls onTabChange when a tab is clicked', () => {
    const onTabChange = vi.fn()
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={onTabChange} accentColor="#3b82f6" />)
    fireEvent.click(screen.getByTestId('tab-settings'))
    expect(onTabChange).toHaveBeenCalledWith('settings')
  })

  it('calls onTabChange on Enter keydown', () => {
    const onTabChange = vi.fn()
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={onTabChange} accentColor="#3b82f6" />)
    fireEvent.keyDown(screen.getByTestId('tab-settings'), { key: 'Enter' })
    expect(onTabChange).toHaveBeenCalledWith('settings')
  })

  it('calls onTabChange on Space keydown', () => {
    const onTabChange = vi.fn()
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={onTabChange} accentColor="#3b82f6" />)
    fireEvent.keyDown(screen.getByTestId('tab-settings'), { key: ' ' })
    expect(onTabChange).toHaveBeenCalledWith('settings')
  })

  it('renders tablist role on container', () => {
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={() => {}} accentColor="#3b82f6" />)
    expect(screen.getByRole('tablist')).toBeInTheDocument()
  })

  it('renders tab role on each tab', () => {
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={() => {}} accentColor="#3b82f6" />)
    expect(screen.getAllByRole('tab')).toHaveLength(2)
  })

  it('has correct aria-label from tooltip', () => {
    render(<FormTabStrip tabs={makeTabs()} activeTabId="chat" onTabChange={() => {}} accentColor="#3b82f6" />)
    expect(screen.getByTestId('tab-chat')).toHaveAttribute('aria-label', 'Chat')
    expect(screen.getByTestId('tab-settings')).toHaveAttribute('aria-label', 'Settings')
  })
})

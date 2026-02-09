import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@/test/render';
import userEvent from '@testing-library/user-event';
import { createElement } from 'react';
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined';
import SettingsOutlined from '@mui/icons-material/SettingsOutlined';
import { TopNavBar } from './TopNavBar';
import type { NavBarItem } from './types';

const makeItem = (overrides: Partial<NavBarItem> = {}): NavBarItem => ({
  key: '/test',
  icon: createElement(SmartToyOutlined, { fontSize: 'small' }),
  label: 'Test Item',
  isActive: false,
  onClick: vi.fn(),
  ...overrides,
});

beforeEach(() => {
  vi.clearAllMocks();
});

describe('TopNavBar', () => {
  it('renders nav items', () => {
    const items = [
      makeItem({ key: '/a', label: 'Alpha' }),
      makeItem({ key: '/b', label: 'Beta' }),
    ];

    render(<TopNavBar navItems={items} utilityItems={[]} />);

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(2);
  });

  it('renders utility items', () => {
    const nav = [makeItem({ key: '/a', label: 'Alpha' })];
    const utility = [
      makeItem({ key: '/settings', label: 'Settings', icon: createElement(SettingsOutlined, { fontSize: 'small' }) }),
    ];

    render(<TopNavBar navItems={nav} utilityItems={utility} />);

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(2);
  });

  it('calls onClick when item is clicked', async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    const items = [makeItem({ key: '/a', label: 'Alpha', onClick })];

    render(<TopNavBar navItems={items} utilityItems={[]} />);

    await user.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('renders badge for items with badge count', () => {
    const items = [makeItem({ key: '/a', label: 'Alpha', badge: 3 })];

    render(<TopNavBar navItems={items} utilityItems={[]} />);

    const badge = document.querySelector('.MuiBadge-badge');
    expect(badge).toBeInTheDocument();
  });

  it('renders trailing content', () => {
    render(
      <TopNavBar
        navItems={[]}
        utilityItems={[]}
        trailing={<button data-testid="trailing">Theme</button>}
      />,
    );

    expect(screen.getByTestId('trailing')).toBeInTheDocument();
  });
});

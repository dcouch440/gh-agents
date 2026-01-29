import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Login } from './Login.js';
import { handleLogin } from './loginHandler.js';

// Mock TextInput to give us direct control over onSubmit
let capturedOnSubmit: ((value: string) => void) | undefined;
vi.mock('ink-text-input', () => ({
  default: ({ onSubmit, value, onChange, mask }: any) => {
    capturedOnSubmit = onSubmit;
    const React = require('react');
    return React.createElement('ink-text', null, mask ? '*'.repeat((value || '').length) : value);
  },
}));

vi.mock('./loginHandler.js', () => ({
  handleLogin: vi.fn(),
}));

const mockedHandleLogin = vi.mocked(handleLogin);

beforeEach(() => {
  vi.clearAllMocks();
  capturedOnSubmit = undefined;
});

describe('Login component', () => {
  it('renders password prompt', () => {
    const { lastFrame } = render(<Login onSuccess={vi.fn()} />);
    expect(lastFrame()).toContain('Password:');
  });

  it('does not submit when value is empty', () => {
    render(<Login onSuccess={vi.fn()} />);
    // Submit empty string should not call handleLogin
    capturedOnSubmit?.('');
    expect(mockedHandleLogin).not.toHaveBeenCalled();
  });

  it('shows loading spinner during authentication', async () => {
    let resolveLogin: (value: { success: boolean }) => void;
    mockedHandleLogin.mockImplementation(
      () => new Promise((resolve) => { resolveLogin = resolve; })
    );

    const { lastFrame } = render(<Login onSuccess={vi.fn()} />);
    capturedOnSubmit?.('secret');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Authenticating');
    });

    resolveLogin!({ success: true });
  });

  it('calls onSuccess when login succeeds', async () => {
    mockedHandleLogin.mockResolvedValue({ success: true });
    const onSuccess = vi.fn();

    render(<Login onSuccess={onSuccess} />);
    capturedOnSubmit?.('secret');

    await vi.waitFor(() => {
      expect(onSuccess).toHaveBeenCalled();
    });
  });

  it('shows error and clears password on login failure', async () => {
    mockedHandleLogin.mockResolvedValue({ success: false, error: 'Invalid password' });
    const onSuccess = vi.fn();

    const { lastFrame } = render(<Login onSuccess={onSuccess} />);
    capturedOnSubmit?.('wrong');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Invalid password');
    });
    expect(onSuccess).not.toHaveBeenCalled();
    // Should be back to password prompt (not loading)
    expect(lastFrame()).toContain('Password:');
  });

  it('shows default error when no error message provided', async () => {
    mockedHandleLogin.mockResolvedValue({ success: false });
    const onSuccess = vi.fn();

    const { lastFrame } = render(<Login onSuccess={onSuccess} />);
    capturedOnSubmit?.('wrong');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Unknown error');
    });
  });
});

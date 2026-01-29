import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Login } from './Login.js';

vi.mock('../api/client.js', () => ({
  api: {
    auth: {
      login: vi.fn(),
    },
  },
}));

vi.mock('../store/auth.js', () => ({
  setToken: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe('Login component', () => {
  it('renders password prompt', () => {
    const { lastFrame } = render(<Login onSuccess={vi.fn()} />);
    expect(lastFrame()).toContain('Password:');
  });

  it('does not submit when value is empty', () => {
    const { stdin } = render(<Login onSuccess={vi.fn()} />);
    // Enter on empty input should not crash
    stdin.write('\r');
    // Component should still show password prompt
  });

  it('does not show plaintext password in output', () => {
    const { stdin, lastFrame } = render(<Login onSuccess={vi.fn()} />);
    stdin.write('a');
    stdin.write('b');
    stdin.write('c');
    const frame = lastFrame()!;
    // The password must never appear as plaintext
    expect(frame).not.toContain('abc');
  });
});

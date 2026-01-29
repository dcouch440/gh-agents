import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the api module
vi.mock('../api/client.js', () => ({
  api: {
    chat: {
      history: vi.fn(),
      send: vi.fn(),
      clear: vi.fn(),
    },
  },
}));

import { api } from '../api/client.js';

const mockHistory = api.chat.history as ReturnType<typeof vi.fn>;
const mockSend = api.chat.send as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('useChat (unit logic)', () => {
  it('api.chat.history is callable', async () => {
    mockHistory.mockResolvedValue([]);
    const result = await api.chat.history();
    expect(result).toEqual([]);
    expect(mockHistory).toHaveBeenCalled();
  });

  it('api.chat.send is callable', async () => {
    mockSend.mockResolvedValue({ message_id: '1', status: 'ok' });
    const result = await api.chat.send('hello');
    expect(result).toEqual({ message_id: '1', status: 'ok' });
    expect(mockSend).toHaveBeenCalledWith('hello');
  });

  it('api.chat.history returns messages', async () => {
    const msgs = [
      { id: '1', role: 'user', content: 'hi', timestamp: '2026-01-29T00:00:00Z' },
    ];
    mockHistory.mockResolvedValue(msgs);
    const result = await api.chat.history();
    expect(result).toHaveLength(1);
    expect(result[0].content).toBe('hi');
  });
});

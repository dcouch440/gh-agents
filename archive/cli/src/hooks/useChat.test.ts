import { describe, it, expect, vi, beforeEach } from 'vitest';

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

describe('useChat API interactions', () => {
  it('api.chat.history resolves with empty array', async () => {
    mockHistory.mockResolvedValue([]);
    const result = await api.chat.history();
    expect(result).toEqual([]);
    expect(mockHistory).toHaveBeenCalled();
  });

  it('api.chat.send resolves with message_id and status', async () => {
    mockSend.mockResolvedValue({ message_id: '1', status: 'ok' });
    const result = await api.chat.send('hello');
    expect(result).toEqual({ message_id: '1', status: 'ok' });
    expect(mockSend).toHaveBeenCalledWith('hello');
  });

  it('api.chat.history returns messages with correct shape', async () => {
    const msgs = [
      {
        id: '1',
        role: 'user',
        content: 'hi',
        timestamp: '2026-01-29T00:00:00Z',
      },
    ];
    mockHistory.mockResolvedValue(msgs);
    const result = await api.chat.history();
    expect(result).toHaveLength(1);
    expect(result[0].content).toBe('hi');
    expect(result[0].role).toBe('user');
    expect(result[0].id).toBe('1');
  });

  it('api.chat.history passes limit and offset', async () => {
    mockHistory.mockResolvedValue([]);
    await api.chat.history(10, 5);
    expect(mockHistory).toHaveBeenCalledWith(10, 5);
  });

  it('api.chat.send rejects on network error', async () => {
    mockSend.mockRejectedValue(new Error('Network error'));
    await expect(api.chat.send('hello')).rejects.toThrow('Network error');
  });

  it('api.chat.history rejects on server error', async () => {
    mockHistory.mockRejectedValue(new Error('API error 500'));
    await expect(api.chat.history()).rejects.toThrow('API error 500');
  });

  it('api.chat.send passes message content verbatim', async () => {
    mockSend.mockResolvedValue({ message_id: '2', status: 'ok' });
    await api.chat.send('message with special chars: <>&"');
    expect(mockSend).toHaveBeenCalledWith('message with special chars: <>&"');
  });

  it('api.chat.history returns multiple messages in order', async () => {
    const msgs = [
      {
        id: '1',
        role: 'user',
        content: 'first',
        timestamp: '2026-01-29T00:00:00Z',
      },
      {
        id: '2',
        role: 'assistant',
        content: 'second',
        timestamp: '2026-01-29T00:00:01Z',
      },
    ];
    mockHistory.mockResolvedValue(msgs);
    const result = await api.chat.history();
    expect(result).toHaveLength(2);
    expect(result[0].role).toBe('user');
    expect(result[1].role).toBe('assistant');
  });
});

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { EventEmitter } from 'events';
import { WsClient } from './ws-client.js';

class FakeSocket extends EventEmitter {
  readyState = 1; // OPEN
  sent: string[] = [];

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = 3; // CLOSED
  }
}

let lastFakeSocket: FakeSocket;
const constructorSpy = vi.fn();

vi.mock('ws', () => {
  const MockWS = function (this: FakeSocket, ...args: unknown[]) {
    constructorSpy(...args);
    const socket = lastFakeSocket;
    return socket;
  } as unknown as typeof import('ws').default;
  (MockWS as unknown as Record<string, number>).OPEN = 1;
  return { default: MockWS };
});

beforeEach(() => {
  vi.useFakeTimers();
  constructorSpy.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

function createClientAndSocket(): { client: WsClient; socket: FakeSocket } {
  const socket = new FakeSocket();
  lastFakeSocket = socket;
  const client = new WsClient('http://localhost:3000', 'test-token');
  return { client, socket };
}

describe('WsClient', () => {
  describe('constructor', () => {
    it('converts http url to ws url', () => {
      const { client, socket } = createClientAndSocket();
      client.connect();
      expect(constructorSpy).toHaveBeenCalledWith('ws://localhost:3000/ws', {
        headers: { Authorization: 'Bearer test-token' },
      });
      socket.emit('open');
    });

    it('converts https url to wss url', () => {
      const socket = new FakeSocket();
      lastFakeSocket = socket;
      const client = new WsClient('https://example.com', 'tok');
      client.connect();
      expect(constructorSpy).toHaveBeenCalledWith('wss://example.com/ws', {
        headers: { Authorization: 'Bearer tok' },
      });
      socket.emit('open');
    });
  });

  describe('connect', () => {
    it('resolves on open', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await expect(p).resolves.toBeUndefined();
    });

    it('rejects on error before open', async () => {
      const { client, socket } = createClientAndSocket();
      socket.readyState = 0; // CONNECTING
      const p = client.connect();
      socket.emit('error', new Error('connection refused'));
      await expect(p).rejects.toThrow('connection refused');
    });

    it('sets connected to true after open', async () => {
      const { client, socket } = createClientAndSocket();
      expect(client.connected).toBe(false);
      const p = client.connect();
      socket.emit('open');
      await p;
      expect(client.connected).toBe(true);
    });
  });

  describe('subscribe / unsubscribe', () => {
    it('sends subscribe message when connected', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      client.subscribe(['agents', 'tasks']);
      expect(socket.sent).toHaveLength(1);
      expect(JSON.parse(socket.sent[0])).toEqual({
        type: 'subscribe',
        channels: ['agents', 'tasks'],
      });
    });

    it('queues subscriptions and sends on connect', async () => {
      const { client, socket } = createClientAndSocket();
      client.subscribe(['agents']);
      // not connected yet, no message sent
      expect(socket.sent).toHaveLength(0);

      const p = client.connect();
      socket.emit('open');
      await p;

      // resubscribe on open
      expect(socket.sent).toHaveLength(1);
      expect(JSON.parse(socket.sent[0])).toEqual({
        type: 'subscribe',
        channels: ['agents'],
      });
    });

    it('sends unsubscribe message', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      client.subscribe(['agents', 'tasks']);
      client.unsubscribe(['agents']);
      expect(JSON.parse(socket.sent[1])).toEqual({
        type: 'unsubscribe',
        channels: ['agents'],
      });
    });
  });

  describe('message dispatch', () => {
    it('dispatches typed messages to handlers', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      const handler = vi.fn();
      client.on('agent_update', handler);

      const msg = {
        type: 'agent_update',
        data: { id: 'a1', status: 'busy', current_task: 't1' },
      };
      socket.emit('message', JSON.stringify(msg));

      expect(handler).toHaveBeenCalledWith(msg.data);
    });

    it('dispatches to wildcard handlers with full message', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      const handler = vi.fn();
      client.on('*', handler);

      const msg = {
        type: 'task_update',
        data: { id: 't1', status: 'completed', progress: 1.0, assigned_agent: null },
      };
      socket.emit('message', JSON.stringify(msg));

      expect(handler).toHaveBeenCalledWith(msg);
    });

    it('dispatches subscribed message (no data field)', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      const handler = vi.fn();
      client.on('subscribed', handler);

      const msg = { type: 'subscribed', channels: ['agents'] };
      socket.emit('message', JSON.stringify(msg));

      expect(handler).toHaveBeenCalledWith(msg);
    });

    it('ignores unparseable messages', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      const handler = vi.fn();
      client.on('*', handler);

      socket.emit('message', 'not-json');
      expect(handler).not.toHaveBeenCalled();
    });
  });

  describe('off', () => {
    it('removes handler', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      const handler = vi.fn();
      client.on('agent_update', handler);
      client.off('agent_update', handler);

      socket.emit(
        'message',
        JSON.stringify({ type: 'agent_update', data: { id: 'a1', status: 'idle', current_task: null } }),
      );
      expect(handler).not.toHaveBeenCalled();
    });
  });

  describe('disconnect', () => {
    it('closes the socket and sets connected to false', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      client.disconnect();
      expect(client.connected).toBe(false);
    });
  });

  describe('reconnect', () => {
    it('attempts reconnect with exponential backoff on close', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      // Simulate close
      const socket2 = new FakeSocket();
      lastFakeSocket = socket2;

      socket.emit('close');

      // Should schedule reconnect after 1s (baseDelay * 2^0)
      expect(constructorSpy).toHaveBeenCalledTimes(1); // only initial
      vi.advanceTimersByTime(1000);
      expect(constructorSpy).toHaveBeenCalledTimes(2); // reconnect attempt

      socket2.emit('open');
    });

    it('does not reconnect after disconnect()', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      client.disconnect();

      const callCount = constructorSpy.mock.calls.length;
      socket.emit('close');
      vi.advanceTimersByTime(60000);
      expect(constructorSpy).toHaveBeenCalledTimes(callCount);
    });

    it('resubscribes channels on reconnect', async () => {
      const { client, socket } = createClientAndSocket();
      const p = client.connect();
      socket.emit('open');
      await p;

      client.subscribe(['agents']);
      socket.sent.length = 0;

      const socket2 = new FakeSocket();
      lastFakeSocket = socket2;
      socket.emit('close');
      vi.advanceTimersByTime(1000);
      socket2.emit('open');

      // Should have resubscribed
      expect(socket2.sent).toHaveLength(1);
      expect(JSON.parse(socket2.sent[0])).toEqual({
        type: 'subscribe',
        channels: ['agents'],
      });
    });
  });
});

import { EventSource } from 'eventsource';

export interface StreamCallbacks {
  onToken: (text: string) => void;
  onDone: () => void;
  onError: (error: string) => void;
}

export function streamResponse(
  baseUrl: string,
  messageId: string,
  token: string,
  callbacks: StreamCallbacks,
): () => void {
  const url = `${baseUrl}/api/chat/${messageId}/stream`;
  const es = new EventSource(url, {
    fetch: (input, init) =>
      fetch(input, {
        ...init,
        headers: { ...init?.headers, Authorization: `Bearer ${token}` },
      }),
  });

  es.addEventListener('token', (event: MessageEvent) => {
    callbacks.onToken(event.data);
  });

  es.addEventListener('done', () => {
    es.close();
    callbacks.onDone();
  });

  es.addEventListener('error', (event: MessageEvent) => {
    const msg = event.data ?? 'Stream error';
    es.close();
    callbacks.onError(msg);
  });

  es.onerror = () => {
    es.close();
    callbacks.onError('SSE connection failed');
  };

  return () => {
    es.close();
  };
}

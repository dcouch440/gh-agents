import { useState, useEffect, useCallback } from 'react';
import { wsClient } from '../api/websocket';

export interface FeedItem {
  id: string;
  agent: string;
  content: string;
  type: 'report' | 'milestone' | 'error' | 'warning' | 'system';
  timestamp: string;
}

export function useFeed() {
  const [items, setItems] = useState<FeedItem[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    let mounted = true;

    wsClient.connect().then(() => {
      if (mounted) {
        setConnected(true);
        wsClient.subscribe(['feed']);
      }
    });

    const handleFeed = (data: unknown) => {
      if (mounted) {
        setItems((prev) => [...prev, data as FeedItem].slice(-200));
      }
    };

    wsClient.on('feed', handleFeed);

    return () => {
      mounted = false;
      wsClient.off('feed', handleFeed);
      wsClient.unsubscribe(['feed']);
    };
  }, []);

  const clear = useCallback(() => {
    setItems([]);
  }, []);

  return { items, connected, clear };
}

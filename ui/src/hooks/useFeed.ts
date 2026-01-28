import { useState, useEffect } from 'react';
import { wsClient } from '../api/websocket';

export interface FeedItem {
  id: string;
  agent: string;
  content: string;
  type: string;
  timestamp: string;
}

export function useFeed() {
  const [items, setItems] = useState<FeedItem[]>([]);

  useEffect(() => {
    wsClient.subscribe(['feed']);

    const handleFeed = (data: unknown) => {
      setItems((prev) => [...prev, data as FeedItem].slice(-100)); // Keep last 100
    };

    wsClient.on('feed', handleFeed);

    return () => {
      wsClient.off('feed', handleFeed);
      wsClient.unsubscribe(['feed']);
    };
  }, []);

  return { items };
}

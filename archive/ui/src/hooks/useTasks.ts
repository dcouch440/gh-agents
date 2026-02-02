import { useState, useEffect, useCallback } from 'react';
import { api, type Task } from '../api/client';
import { wsClient } from '../api/websocket';

export function useTasks() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Load initial tasks
    api.tasks.list().then((taskList) => {
      setTasks(taskList);
      setLoading(false);
    });

    // Subscribe to task updates
    wsClient.subscribe(['tasks']);

    const handleTaskUpdate = (data: unknown) => {
      const updatedTask = data as Task;
      setTasks((prev) => {
        const index = prev.findIndex((t) => t.id === updatedTask.id);
        if (index >= 0) {
          const newTasks = [...prev];
          newTasks[index] = updatedTask;
          return newTasks;
        }
        return [...prev, updatedTask];
      });
    };

    wsClient.on('task_update', handleTaskUpdate);

    return () => {
      wsClient.off('task_update', handleTaskUpdate);
      wsClient.unsubscribe(['tasks']);
    };
  }, []);

  const createTask = useCallback(async (title: string, description: string) => {
    const newTask = await api.tasks.create({ title, description });
    setTasks((prev) => [...prev, newTask]);
    return newTask;
  }, []);

  const refreshTasks = useCallback(async () => {
    setLoading(true);
    const taskList = await api.tasks.list();
    setTasks(taskList);
    setLoading(false);
  }, []);

  return { tasks, loading, createTask, refreshTasks };
}

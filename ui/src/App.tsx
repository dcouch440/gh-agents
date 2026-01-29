import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { api } from './api/client';
import { useAuthStore } from './store';
import { LoginPage } from './pages/LoginPage';
import { SetupPage } from './pages/SetupPage';
import { Layout } from './components/Layout';
import { ChatPage } from './pages/ChatPage';
import { FeedPage } from './pages/FeedPage';
import { TasksPage } from './pages/TasksPage';
import { AgentsPage } from './pages/AgentsPage';
import { FilesPage } from './pages/FilesPage';
import { StatsPage } from './pages/StatsPage';
import { SettingsPage } from './pages/SettingsPage';

function App() {
  const [needsSetup, setNeedsSetup] = useState<boolean | null>(null);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

  useEffect(() => {
    let mounted = true;

    api
      .health()
      .then(() => api.auth.me())
      .then(() => {
        if (mounted) {
          setNeedsSetup(false);
        }
      })
      .catch((err) => {
        if (mounted) {
          if (err.status === 404) {
            setNeedsSetup(true);
          } else {
            setNeedsSetup(false);
          }
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  if (needsSetup === null) {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: 'var(--bg-primary)',
        }}
      >
        <div style={{ color: 'var(--text-secondary)' }}>Loading...</div>
      </div>
    );
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route
          path="/setup"
          element={needsSetup ? <SetupPage /> : <Navigate to="/login" replace />}
        />
        <Route
          path="/login"
          element={
            isAuthenticated ? (
              <Navigate to="/" replace />
            ) : needsSetup ? (
              <Navigate to="/setup" replace />
            ) : (
              <LoginPage />
            )
          }
        />
        <Route
          path="/"
          element={
            isAuthenticated ? (
              <Layout />
            ) : needsSetup ? (
              <Navigate to="/setup" replace />
            ) : (
              <Navigate to="/login" replace />
            )
          }
        >
          <Route index element={<Navigate to="/chat" replace />} />
          <Route path="chat" element={<ChatPage />} />
          <Route path="feed" element={<FeedPage />} />
          <Route path="tasks" element={<TasksPage />} />
          <Route path="agents" element={<AgentsPage />} />
          <Route path="files" element={<FilesPage />} />
          <Route path="stats" element={<StatsPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;

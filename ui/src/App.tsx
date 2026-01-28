import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { api } from './api/client';
import { useAuthStore } from './store';
import { LoginPage } from './pages/LoginPage';
import { SetupPage } from './pages/SetupPage';

// Placeholder until Layout component is built in 11.4
const Layout = () => <div style={{ padding: '2rem', color: 'var(--text-primary)' }}>Dashboard</div>;

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
          path="/*"
          element={
            isAuthenticated ? (
              <Layout />
            ) : needsSetup ? (
              <Navigate to="/setup" replace />
            ) : (
              <Navigate to="/login" replace />
            )
          }
        />
      </Routes>
    </BrowserRouter>
  );
}

export default App;

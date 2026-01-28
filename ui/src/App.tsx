import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';

// Placeholder pages (will be replaced)
const LoginPage = () => <div className="p-8">Login</div>;
const DashboardPage = () => <div className="p-8">Dashboard</div>;

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/" element={<DashboardPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;

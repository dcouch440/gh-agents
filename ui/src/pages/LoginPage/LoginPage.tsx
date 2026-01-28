import { useState, FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../../api/client';
import { useAuthStore } from '../../store';
import { Input } from '../../components/Input';
import { Button } from '../../components/Button';
import styles from './LoginPage.module.css';

export function LoginPage() {
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const setToken = useAuthStore((state) => state.setToken);

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const { token, expires_in } = await api.auth.login(password);
      setToken(token, expires_in);
      navigate('/');
    } catch (err) {
      setError('Invalid password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.container}>
      <div className={styles.card}>
        <div className={styles.header}>
          <h1 className={styles.title}>nexor</h1>
          <p className={styles.subtitle}>AI Agent Orchestration</p>
        </div>

        <form onSubmit={handleSubmit} className={styles.form}>
          <Input
            id="password"
            type="password"
            label="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Enter your password"
            error={error}
            autoFocus
          />

          <Button
            type="submit"
            disabled={!password}
            isLoading={loading}
            fullWidth
          >
            Sign in
          </Button>
        </form>
      </div>
    </div>
  );
}

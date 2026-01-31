import {useState, FormEvent} from "react";
import {useNavigate} from "react-router-dom";
import {api} from "../../api/client";
import {Input} from "../../components/Input";
import {Button} from "../../components/Button";
import styles from "./SetupPage.module.css";

export function SetupPage() {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError("");

    // Validation
    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }
    if (password !== confirm) {
      setError("Passwords do not match");
      return;
    }

    setLoading(true);

    try {
      await api.auth.setup(password);
      navigate("/login");
    } catch {
      setError("Failed to set up password");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.container}>
      <div className={styles.card}>
        <div className={styles.header}>
          <h1 className={styles.title}>nexor</h1>
          <p className={styles.subtitle}>First Time Setup</p>
        </div>

        <form onSubmit={handleSubmit} className={styles.form}>
          <p className={styles.description}>
            Create a password to protect your nexor instance. This password is
            stored locally and never sent anywhere.
          </p>

          <Input
            id="password"
            type="password"
            label="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="At least 8 characters"
            autoFocus
          />

          <Input
            id="confirm"
            type="password"
            label="Confirm Password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            placeholder="Confirm your password"
          />

          {error && <p className={styles.error}>{error}</p>}

          <Button
            type="submit"
            disabled={!password || !confirm}
            isLoading={loading}
            fullWidth
          >
            Create Password
          </Button>
        </form>
      </div>
    </div>
  );
}

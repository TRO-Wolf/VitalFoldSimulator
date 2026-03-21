import { h } from 'https://esm.sh/preact@10';
import { useState } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import * as api from '../api.js';

const html = htm.bind(h);

export function LoginPage({ onLogin }) {
  const [mode, setMode] = useState('login');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [username, setUsername] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e) {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      let data;
      if (mode === 'admin') {
        data = await api.post('/api/v1/auth/admin-login', { username, password });
      } else if (mode === 'register') {
        data = await api.post('/api/v1/auth/register', { email, password });
      } else {
        data = await api.post('/api/v1/auth/login', { email, password });
      }
      api.setAuth(data.token, data.user);
      onLogin(data.user);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  function switchMode(newMode) {
    setMode(newMode);
    setError('');
  }

  const isAdmin = mode === 'admin';
  const buttonLabel = mode === 'register' ? 'Create Account' : mode === 'admin' ? 'Admin Login' : 'Sign In';

  return html`
    <div class="login-wrapper">
      <article>
        <header>
          <hgroup>
            <h2>VitalFold Engine</h2>
            <p>Synthetic Health Data Simulator</p>
          </hgroup>
        </header>

        ${!isAdmin && html`
          <div class="tab-bar">
            <button type="button" class=${mode === 'login' ? '' : 'outline'}
                    onclick=${() => switchMode('login')}>Login</button>
            <button type="button" class=${mode === 'register' ? '' : 'outline'}
                    onclick=${() => switchMode('register')}>Register</button>
          </div>
        `}

        <form onsubmit=${handleSubmit}>
          ${isAdmin ? html`
            <label>
              Username
              <input type="text" value=${username}
                     onInput=${e => setUsername(e.target.value)}
                     placeholder="admin"
                     required />
            </label>
          ` : html`
            <label>
              Email
              <input type="email" value=${email}
                     onInput=${e => setEmail(e.target.value)}
                     placeholder="user@example.com"
                     required />
            </label>
          `}

          <label>
            Password
            <input type="password" value=${password}
                   onInput=${e => setPassword(e.target.value)}
                   placeholder="Enter password"
                   required
                   minlength="8" />
          </label>

          <button type="submit" aria-busy=${loading} disabled=${loading}>
            ${loading ? 'Please wait...' : buttonLabel}
          </button>

          ${error && html`<p class="error-msg">${error}</p>`}
        </form>

        <div class="admin-link">
          ${isAdmin
            ? html`<a href="#" onclick=${(e) => { e.preventDefault(); switchMode('login'); }}>Back to user login</a>`
            : html`<a href="#" onclick=${(e) => { e.preventDefault(); switchMode('admin'); }}>Admin? Login here</a>`
          }
        </div>
      </article>
    </div>
  `;
}

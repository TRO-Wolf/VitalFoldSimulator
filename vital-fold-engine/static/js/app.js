import { h, render } from 'https://esm.sh/preact@10';
import { useState, useEffect } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import * as api from './api.js';
import { Nav } from './components/nav.js';
import { LoginPage } from './pages/login.js';
import { DashboardPage } from './pages/dashboard.js';
import { VisitorsPage } from './pages/visitors.js';

const html = htm.bind(h);
const PROTECTED_ROUTES = ['#/dashboard', '#/visitors'];

function App() {
  const [user, setUser] = useState(api.getUser());
  const [route, setRoute] = useState(window.location.hash || '#/login');

  // Listen for hash changes
  useEffect(() => {
    function onHashChange() {
      setRoute(window.location.hash || '#/login');
    }
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  // Redirect based on auth state
  useEffect(() => {
    if (api.isAuthenticated() && (route === '#/login' || route === '' || route === '#/')) {
      window.location.hash = '#/dashboard';
    } else if (!api.isAuthenticated() && PROTECTED_ROUTES.includes(route)) {
      window.location.hash = '#/login';
    }
  }, [route, user]);

  function handleLogin(userData) {
    setUser(userData);
    window.location.hash = '#/dashboard';
  }

  function handleLogout() {
    api.clearAuth();
    setUser(null);
    window.location.hash = '#/login';
  }

  const isLoggedIn = api.isAuthenticated();
  let page;
  if (!isLoggedIn) {
    page = html`<${LoginPage} onLogin=${handleLogin} />`;
  } else if (route === '#/visitors') {
    page = html`<${VisitorsPage} />`;
  } else {
    page = html`<${DashboardPage} />`;
  }

  return html`
    <${Nav} email=${isLoggedIn ? user?.email : null} route=${route} onLogout=${handleLogout} />
    ${page}
  `;
}

render(html`<${App} />`, document.getElementById('app'));

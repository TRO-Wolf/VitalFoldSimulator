import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

export function Nav({ email, route, onLogout }) {
  return html`
    <nav class="container-fluid">
      <ul>
        <li><strong>VitalFold Engine</strong></li>
        ${email && html`
          <li><a href="#/dashboard" class=${route === '#/dashboard' ? 'nav-active' : ''}>Dashboard</a></li>
          <li><a href="#/visitors" class=${route === '#/visitors' ? 'nav-active' : ''}>Visitors</a></li>
        `}
      </ul>
      ${email && html`
        <ul>
          <li class="nav-right">
            <span>${email}</span>
            <button class="outline secondary" onclick=${onLogout}>Logout</button>
          </li>
        </ul>
      `}
    </nav>
  `;
}

import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

function formatTimestamp(iso) {
  if (!iso) return 'Never';
  const d = new Date(iso);
  return d.toLocaleString();
}

export function StatusBadge({ running, lastRun }) {
  return html`
    <article>
      <header>Status</header>
      <div class="status-row">
        <span class="dot ${running ? 'dot--running' : 'dot--idle'}"></span>
        <strong>${running ? 'Running' : 'Idle'}</strong>
      </div>
      <small>Last run: ${formatTimestamp(lastRun)}</small>
    </article>
  `;
}

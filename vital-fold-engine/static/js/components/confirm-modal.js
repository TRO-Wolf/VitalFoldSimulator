import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

export function ConfirmModal({ open, title, message, onConfirm, onCancel }) {
  if (!open) return null;

  return html`
    <div class="modal-overlay" onclick=${onCancel}>
      <article onclick=${e => e.stopPropagation()}>
        <header>${title}</header>
        <p>${message}</p>
        <footer>
          <button class="secondary" onclick=${onCancel}>Cancel</button>
          <button class="btn-danger" onclick=${onConfirm}>Confirm Reset</button>
        </footer>
      </article>
    </div>
  `;
}

import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

export function CountTable({ title, counts }) {
  return html`
    <article>
      <header>${title}</header>
      <table>
        <tbody>
          ${counts.map(({ label, value }) => html`
            <tr key=${label}>
              <td>${label}</td>
              <td class="count-value">${(value || 0).toLocaleString()}</td>
            </tr>
          `)}
        </tbody>
      </table>
    </article>
  `;
}

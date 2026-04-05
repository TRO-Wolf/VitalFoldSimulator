import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

export function DateRangeForm({ config, onChange, disabled, onSubmit, loading }) {
  function handleDateInput(key, value) {
    onChange({ ...config, [key]: value });
  }

  return html`
    <article>
      <header>DynamoDB Sync</header>
      <p style="margin-bottom: 0.75rem; color: var(--pico-muted-color); font-size: 0.875rem;">
        Sync existing Aurora visit data to DynamoDB for a date range.
        Requires a prior Dynamic Populate run.
      </p>
      <div class="grid">
        <label>
          Start Date
          <input type="date"
                 value=${config.start_date}
                 onInput=${e => handleDateInput('start_date', e.target.value)}
                 disabled=${disabled} />
        </label>
        <label>
          End Date
          <input type="date"
                 value=${config.end_date}
                 onInput=${e => handleDateInput('end_date', e.target.value)}
                 disabled=${disabled} />
        </label>
      </div>
      <button onclick=${onSubmit}
              disabled=${disabled || loading}
              aria-busy=${loading}>
        Sync to DynamoDB
      </button>
    </article>
  `;
}

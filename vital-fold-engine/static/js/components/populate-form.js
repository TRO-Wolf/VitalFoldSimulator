import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

const FIELDS = [
  { key: 'providers', label: 'Providers', default: 50 },
  { key: 'patients', label: 'Patients', default: 50000 },
  { key: 'plans_per_company', label: 'Plans / Company', default: 3 },
  { key: 'appointments_per_patient', label: 'Appts / Patient', default: 2 },
  { key: 'records_per_appointment', label: 'Records / Appt', default: 1 },
];

export function PopulateForm({ config, onChange, disabled }) {
  function handleInput(key, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      onChange({ ...config, [key]: num });
    }
  }

  return html`
    <article>
      <header>Populate Configuration</header>
      <div class="grid">
        ${FIELDS.slice(0, 2).map(f => html`
          <label key=${f.key}>
            ${f.label}
            <input type="number" min="1"
                   value=${config[f.key]}
                   onInput=${e => handleInput(f.key, e.target.value)}
                   disabled=${disabled} />
          </label>
        `)}
      </div>
      <div class="grid">
        ${FIELDS.slice(2).map(f => html`
          <label key=${f.key}>
            ${f.label}
            <input type="number" min="1"
                   value=${config[f.key]}
                   onInput=${e => handleInput(f.key, e.target.value)}
                   disabled=${disabled} />
          </label>
        `)}
      </div>
    </article>
  `;
}

export { FIELDS as POPULATE_DEFAULTS };

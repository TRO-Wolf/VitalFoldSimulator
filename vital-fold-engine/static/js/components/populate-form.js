import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

const FIELDS = [
  { key: 'providers', label: 'Providers', default: 50 },
  { key: 'patients', label: 'Patients', default: 50000 },
  { key: 'plans_per_company', label: 'Plans / Company', default: 3 },
];

export function PopulateForm({ config, onChange, disabled, onSubmit, loading, hasStaticData }) {
  function handleInput(key, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      onChange({ ...config, [key]: num });
    }
  }

  return html`
    <article>
      <header>Static Populate (Step 1)</header>
      <p style="margin-bottom: 0.75rem; color: var(--pico-muted-color); font-size: 0.875rem;">
        Generate reference data: insurance companies, plans, clinics, providers,
        patients, emergency contacts, demographics, and insurance links.
      </p>
      <div class="grid">
        ${FIELDS.map(f => html`
          <label key=${f.key}>
            ${f.label}
            <input type="number" min="1"
                   value=${config[f.key]}
                   onInput=${e => handleInput(f.key, e.target.value)}
                   disabled=${disabled || hasStaticData} />
          </label>
        `)}
      </div>
      ${hasStaticData && html`
        <p style="color: var(--pico-ins-color); font-size: 0.875rem; margin-bottom: 0.5rem;">
          Static data already populated. Reset Aurora to re-run.
        </p>
      `}
      <button onclick=${onSubmit}
              disabled=${disabled || loading || hasStaticData}
              aria-busy=${loading}>
        Populate Static Data
      </button>
    </article>
  `;
}

export { FIELDS as POPULATE_DEFAULTS };

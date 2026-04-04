import { h } from 'https://esm.sh/preact@10';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

const FIELDS = [
  { key: 'providers', label: 'Providers', default: 50 },
  { key: 'patients', label: 'Patients', default: 50000 },
  { key: 'plans_per_company', label: 'Plans / Company', default: 3 },
];

const CLINIC_LABELS = [
  'Charlotte, NC', 'Asheville, NC',
  'Atlanta 1, GA', 'Atlanta 2, GA',
  'Tallahassee, FL',
  'Miami 1, FL', 'Miami 2, FL',
  'Orlando, FL',
  'Jacksonville 1, FL', 'Jacksonville 2, FL',
];

export function PopulateForm({ config, onChange, disabled, onSubmit, loading, hasStaticData }) {
  function handleInput(key, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      onChange({ ...config, [key]: num });
    }
  }

  function handleWeightInput(idx, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      const weights = [...config.clinic_weights];
      weights[idx] = num;
      onChange({ ...config, clinic_weights: weights });
    }
  }

  return html`
    <article>
      <header>Static Populate (Step 1)</header>
      <p style="margin-bottom: 0.75rem; color: var(--pico-muted-color); font-size: 0.875rem;">
        Generate reference data: insurance, clinics, providers, patients, demographics.
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

      <h6 style="margin: 1rem 0 0.25rem; font-size: 0.85rem; color: var(--pico-muted-color);">
        Clinic Weights
      </h6>
      <p style="font-size: 0.8rem; color: var(--pico-muted-color); margin-bottom: 0.5rem;">
        Higher weight = more patients and providers at that clinic.
      </p>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.25rem 1rem;">
        ${config.clinic_weights && config.clinic_weights.map((w, i) => html`
          <label key=${i} style="font-size: 0.85rem; margin-bottom: 0.25rem;">
            ${CLINIC_LABELS[i]}
            <input type="number" min="1" style="padding: 0.3rem 0.5rem;"
                   value=${w}
                   onInput=${e => handleWeightInput(i, e.target.value)}
                   disabled=${disabled || hasStaticData} />
          </label>
        `)}
      </div>

      ${hasStaticData && html`
        <p style="color: var(--pico-ins-color); font-size: 0.875rem; margin: 0.75rem 0 0.5rem;">
          Static data already populated. Reset Aurora to re-run.
        </p>
      `}
      <button onclick=${onSubmit}
              disabled=${disabled || loading || hasStaticData}
              aria-busy=${loading}
              style="margin-top: 0.75rem;">
        Populate Static Data
      </button>
    </article>
  `;
}

export { FIELDS as POPULATE_DEFAULTS };

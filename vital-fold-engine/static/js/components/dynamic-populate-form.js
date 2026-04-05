import { h } from 'https://esm.sh/preact@10';
import { useState } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import { PopulateCalendar } from './populate-calendar.js';

const html = htm.bind(h);

const CLINIC_LABELS = [
  'Charlotte, NC', 'Asheville, NC',
  'Atlanta 1, GA', 'Atlanta 2, GA',
  'Tallahassee, FL',
  'Miami 1, FL', 'Miami 2, FL',
  'Orlando, FL',
  'Jacksonville 1, FL', 'Jacksonville 2, FL',
];

export function DynamicPopulateForm({ config, onChange, disabled, onSubmit, loading, populatedDates, hasStaticData }) {
  // Track which date the next click sets: 'start' or 'end'
  const [selectingDate, setSelectingDate] = useState('start');

  function handleWeightInput(idx, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      const weights = [...config.clinic_weights];
      weights[idx] = num;
      onChange({ ...config, clinic_weights: weights });
    }
  }

  function handleDateInput(key, value) {
    onChange({ ...config, [key]: value });
  }

  function handleNumberInput(key, value) {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      onChange({ ...config, [key]: num });
    }
  }

  function handleCalendarSelect(dateStr) {
    if (selectingDate === 'start') {
      // If the selected start is after current end, also move end
      const newConfig = { ...config, start_date: dateStr };
      if (dateStr > config.end_date) {
        newConfig.end_date = dateStr;
      }
      onChange(newConfig);
      setSelectingDate('end');
    } else {
      // Setting end date — if before start, swap them
      if (dateStr < config.start_date) {
        onChange({ ...config, start_date: dateStr, end_date: config.start_date });
      } else {
        onChange({ ...config, end_date: dateStr });
      }
      setSelectingDate('start');
    }
  }

  // Check for overlap between selected range and populated dates
  let overlapCount = 0;
  if (populatedDates && populatedDates.size > 0 && config.start_date && config.end_date) {
    const start = new Date(config.start_date + 'T00:00:00');
    const end = new Date(config.end_date + 'T00:00:00');
    const cursor = new Date(start);
    while (cursor <= end) {
      const ds = `${cursor.getFullYear()}-${String(cursor.getMonth() + 1).padStart(2, '0')}-${String(cursor.getDate()).padStart(2, '0')}`;
      if (populatedDates.has(ds)) overlapCount++;
      cursor.setDate(cursor.getDate() + 1);
    }
  }

  const hasOverlap = overlapCount > 0;
  const formDisabled = disabled || !hasStaticData;

  return html`
    <article>
      <header>Dynamic Populate (Step 2)</header>

      ${!hasStaticData && html`
        <p style="color: var(--pico-del-color); font-size: 0.875rem; margin-bottom: 0.75rem;">
          Run Static Populate (Step 1) first to generate reference data.
        </p>
      `}

      ${hasStaticData && html`
        <p style="margin-bottom: 0.75rem; color: var(--pico-muted-color); font-size: 0.875rem;">
          Select a date range on the calendar, then configure volume below.
          Green = already populated. Red = overlap conflict.
        </p>
      `}

      <${PopulateCalendar}
        populatedDates=${populatedDates}
        selectedStart=${config.start_date}
        selectedEnd=${config.end_date}
        onSelectDate=${handleCalendarSelect}
        disabled=${formDisabled}
      />

      <div class="date-range-display">
        <label class=${selectingDate === 'start' ? 'date-range-active' : ''}>
          Start Date
          <input type="date"
                 value=${config.start_date}
                 onInput=${e => handleDateInput('start_date', e.target.value)}
                 onFocus=${() => setSelectingDate('start')}
                 disabled=${formDisabled} />
        </label>
        <span class="date-range-arrow">\u2192</span>
        <label class=${selectingDate === 'end' ? 'date-range-active' : ''}>
          End Date
          <input type="date"
                 value=${config.end_date}
                 onInput=${e => handleDateInput('end_date', e.target.value)}
                 onFocus=${() => setSelectingDate('end')}
                 disabled=${formDisabled} />
        </label>
      </div>

      <div class="grid">
        <label>
          Records / Appt
          <input type="number" min="1"
                 value=${config.records_per_appointment}
                 onInput=${e => handleNumberInput('records_per_appointment', e.target.value)}
                 disabled=${formDisabled} />
        </label>
      </div>
      <p style="font-size: 0.8rem; color: var(--pico-muted-color); margin-top: 0.25rem;">
        Appointments are auto-calculated: 36 slots/day per provider, distributed by clinic weight.
      </p>

      <h6 style="margin: 1rem 0 0.25rem; font-size: 0.85rem; color: var(--pico-muted-color);">
        Clinic Weights
      </h6>
      <p style="font-size: 0.8rem; color: var(--pico-muted-color); margin-bottom: 0.5rem;">
        Higher weight = more appointments at that clinic per day.
      </p>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.25rem 1rem;">
        ${config.clinic_weights && config.clinic_weights.map((w, i) => html`
          <label key=${i} style="font-size: 0.85rem; margin-bottom: 0.25rem;">
            ${CLINIC_LABELS[i]}
            <input type="number" min="1" style="padding: 0.3rem 0.5rem;"
                   value=${w}
                   onInput=${e => handleWeightInput(i, e.target.value)}
                   disabled=${formDisabled} />
          </label>
        `)}
      </div>

      ${hasOverlap && html`
        <p style="color: var(--pico-del-color); font-size: 0.875rem; margin-top: 0.5rem;">
          Warning: Selected range overlaps ${overlapCount} already-populated date(s).
          Reset dynamic data or choose a non-overlapping range.
        </p>
      `}

      <button onclick=${onSubmit}
              disabled=${formDisabled || loading || hasOverlap}
              aria-busy=${loading}
              style="margin-top: 0.75rem;">
        Populate Date Range
      </button>
    </article>
  `;
}

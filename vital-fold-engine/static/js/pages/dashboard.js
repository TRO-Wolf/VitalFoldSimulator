import { h } from 'https://esm.sh/preact@10';
import { useState, useEffect, useRef } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import * as api from '../api.js';
import { StatusBadge } from '../components/status-badge.js';
import { CountTable } from '../components/count-table.js';
import { PopulateForm } from '../components/populate-form.js';
import { DynamicPopulateForm } from '../components/dynamic-populate-form.js';
import { ConfirmModal } from '../components/confirm-modal.js';
import { Heatmap } from '../components/heatmap.js';
import { DateRangeForm } from '../components/date-range-form.js';

const html = htm.bind(h);

const AURORA_FIELDS = [
  { key: 'insurance_companies', label: 'Insurance Companies' },
  { key: 'insurance_plans', label: 'Insurance Plans' },
  { key: 'clinics', label: 'Clinics' },
  { key: 'providers', label: 'Providers' },
  { key: 'patients', label: 'Patients' },
  { key: 'emergency_contacts', label: 'Emergency Contacts' },
  { key: 'patient_demographics', label: 'Demographics' },
  { key: 'patient_insurance', label: 'Patient Insurance' },
  { key: 'clinic_schedules', label: 'Clinic Schedules' },
  { key: 'appointments', label: 'Appointments' },
  { key: 'medical_records', label: 'Medical Records' },
  { key: 'patient_visits', label: 'Patient Visits' },
  { key: 'patient_vitals', label: 'Patient Vitals' },
];

const DYNAMO_FIELDS = [
  { key: 'dynamo_patient_visits', label: 'Patient Visits' },
  { key: 'dynamo_patient_vitals', label: 'Patient Vitals' },
];

export function DashboardPage() {
  const [status, setStatus] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState('');
  const [actionLoading, setActionLoading] = useState('');
  const [confirmAction, setConfirmAction] = useState(null);
  const [timelapse, setTimelapse] = useState(null);
  const [populatedDates, setPopulatedDates] = useState(new Set());
  const [dbCounts, setDbCounts] = useState(null);
  const [dbCountsLoading, setDbCountsLoading] = useState(false);

  // Default per-clinic weights (matches DEFAULT_CLINIC_WEIGHTS in Rust)
  const DEFAULT_WEIGHTS = [12, 3, 14, 14, 2, 14, 14, 12, 8, 8];

  // Static populate config (Step 1)
  const [staticConfig, setStaticConfig] = useState({
    providers: 50,
    patients: 50000,
    plans_per_company: 3,
    clinic_weights: [...DEFAULT_WEIGHTS],
  });

  // Dynamic populate config (Step 2)
  const [dynamicConfig, setDynamicConfig] = useState(() => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const ninetyOut = new Date();
    ninetyOut.setDate(ninetyOut.getDate() + 90);
    return {
      start_date: tomorrow.toISOString().split('T')[0],
      end_date: ninetyOut.toISOString().split('T')[0],
      records_per_appointment: 1,
      clinic_weights: [...DEFAULT_WEIGHTS],
    };
  });

  // Date range simulation config (separate from populate)
  const [dateRangeConfig, setDateRangeConfig] = useState(() => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const tomorrowStr = tomorrow.toISOString().split('T')[0];
    return {
      start_date: tomorrowStr,
      end_date: tomorrowStr,
    };
  });

  const intervalRef = useRef(null);
  const heatmapRef = useRef(null);

  async function fetchStatus() {
    try {
      const data = await api.get('/simulate/status');
      setStatus(data);
      setRunning(data.running);
    } catch (err) {
      if (err.message !== 'Session expired') {
        setError(err.message);
      }
    }
  }

  async function fetchPopulatedDates() {
    try {
      const dates = await api.get('/populate/dates');
      setPopulatedDates(new Set(dates));
    } catch (_) { /* ignore — non-critical */ }
  }

  // Poll for status
  useEffect(() => {
    fetchStatus();
    fetchPopulatedDates();

    function startPolling() {
      if (intervalRef.current) clearInterval(intervalRef.current);
      intervalRef.current = setInterval(fetchStatus, running ? 2000 : 10000);
    }

    startPolling();
    return () => clearInterval(intervalRef.current);
  }, [running]);

  // Poll for heatmap data when running
  useEffect(() => {
    if (!running) return;

    async function fetchHeatmap() {
      try {
        const data = await api.get('/simulate/heatmap');
        if (data && data.clinics) setTimelapse(data);
      } catch (_) { /* ignore */ }
    }

    fetchHeatmap();
    heatmapRef.current = setInterval(fetchHeatmap, 2000);
    return () => clearInterval(heatmapRef.current);
  }, [running]);

  async function handleAction(action, path, body) {
    setError('');
    setActionLoading(action);
    setDbCounts(null);
    try {
      await api.post(path, body);
      await fetchStatus();
    } catch (err) {
      setError(err.message);
    } finally {
      setActionLoading('');
    }
  }

  function handlePopulateStatic() {
    handleAction('populate-static', '/populate/static', staticConfig);
  }

  async function handlePopulateDynamic() {
    await handleAction('populate-dynamic', '/populate/dynamic', dynamicConfig);
    fetchPopulatedDates();
  }

  function handleSimulate() {
    handleAction('simulate', '/simulate');
  }

  function handleStop() {
    handleAction('stop', '/simulate/stop');
  }

  function handleDateRangeSimulate() {
    handleAction('date-range', '/simulate/date-range', dateRangeConfig);
  }

  function handleTimelapse() {
    handleAction('timelapse', '/simulate/timelapse', { window_interval_secs: 5 });
  }

  async function handleRefreshDbCounts() {
    setDbCountsLoading(true);
    try {
      const counts = await api.get('/simulate/db-counts');
      setDbCounts(counts);
    } catch (err) {
      setError(err.message);
    } finally {
      setDbCountsLoading(false);
    }
  }

  function requestReset(type) {
    if (type === 'aurora') {
      setConfirmAction({
        title: 'Reset All Aurora DSQL Data',
        message: 'This will permanently delete ALL generated Aurora DSQL data (static + dynamic). This cannot be undone.',
        action: async () => {
          await handleAction('reset', '/simulate/reset');
          fetchPopulatedDates();
        },
      });
    } else if (type === 'dynamo') {
      setConfirmAction({
        title: 'Reset DynamoDB Data',
        message: 'This will permanently delete all DynamoDB patient visit data. This cannot be undone.',
        action: () => handleAction('reset-dynamo', '/simulate/reset-dynamo'),
      });
    } else if (type === 'dynamic') {
      setConfirmAction({
        title: 'Reset Dynamic Data',
        message: 'This will delete clinic schedules, appointments, medical records, and visits. Static reference data (patients, providers, etc.) will be preserved.',
        action: async () => {
          await handleAction('reset-dynamic', '/populate/reset-dynamic');
          fetchPopulatedDates();
        },
      });
    } else if (type === 'init-db') {
      setConfirmAction({
        title: 'Initialize Database Schema',
        message: 'This will DROP and recreate the entire vital_fold schema. All simulation data (Aurora + in-memory counts) will be lost. The public.users auth table is preserved. Continue?',
        action: async () => {
          setActionLoading('init-db');
          try {
            await api.post('/admin/init-db');
            await fetchStatus();
            fetchPopulatedDates();
          } catch (err) {
            setError(err.message);
          } finally {
            setActionLoading('');
          }
        },
      });
    }
  }

  async function confirmReset() {
    if (confirmAction) {
      await confirmAction.action();
      setConfirmAction(null);
    }
  }

  const resetProgress = status?.reset_progress || null;
  const populateProgress = status?.populate_progress || null;
  const dynamoProgress = status?.dynamo_progress || null;
  const hasStaticData = (status?.patients || 0) > 0;

  // Build count arrays — prefer live DB counts when available, else in-memory status
  const countSource = dbCounts || status;
  const auroraCounts = AURORA_FIELDS.map(f => ({
    label: f.label,
    value: countSource ? countSource[f.key] : 0,
  }));

  const dynamoCounts = DYNAMO_FIELDS.map(f => ({
    label: f.label,
    value: countSource ? countSource[f.key] : 0,
  }));

  return html`
    <main class="container">
      ${error && html`<p class="error-msg">${error}</p>`}

      <div class="dashboard-grid">
        <${StatusBadge} running=${running} lastRun=${status?.last_run} />

        <article>
          <header>Controls</header>
          <div class="controls-group">
            <button onclick=${handleSimulate}
                    disabled=${running || actionLoading === 'simulate'}
                    aria-busy=${actionLoading === 'simulate'}>
              Simulate
            </button>
            <button onclick=${handleTimelapse}
                    disabled=${running || actionLoading === 'timelapse'}
                    aria-busy=${actionLoading === 'timelapse'}>
              Heatmap
            </button>
            <button class="secondary" onclick=${handleStop}
                    disabled=${!running || actionLoading === 'stop'}
                    aria-busy=${actionLoading === 'stop'}>
              Stop
            </button>
          </div>
          <div class="controls-group" style="margin-top: 0.75rem;">
            <button class="outline btn-danger" onclick=${() => requestReset('aurora')}
                    disabled=${running}>
              Reset Aurora
            </button>
            <button class="outline btn-danger" onclick=${() => requestReset('dynamic')}
                    disabled=${running}>
              Reset Dynamic
            </button>
            <button class="outline btn-danger" onclick=${() => requestReset('dynamo')}
                    disabled=${running}>
              Reset DynamoDB
            </button>
            <button class="outline btn-danger" onclick=${() => requestReset('init-db')}
                    disabled=${running}
                    aria-busy=${actionLoading === 'init-db'}>
              Init Database
            </button>
          </div>
        </article>
      </div>

      ${resetProgress && html`
        <article class="reset-progress">
          <header>${resetProgress.is_complete ? 'Reset Complete' : 'Resetting Aurora DSQL'}</header>
          <progress
            value=${resetProgress.tables_done}
            max=${resetProgress.total_tables}
          />
          <div class="reset-progress-details">
            <span>
              ${resetProgress.is_complete
                ? html`<span class="reset-complete">All tables cleared</span>`
                : html`Deleting: ${resetProgress.current_table}`}
            </span>
            <span>${resetProgress.tables_done} / ${resetProgress.total_tables} tables</span>
            <span>${resetProgress.rows_deleted.toLocaleString()} rows deleted</span>
          </div>
        </article>
      `}

      ${populateProgress && html`
        <article class="populate-progress">
          <header>${populateProgress.is_complete ? 'Populate Complete' : 'Populating Aurora DSQL'}</header>
          <progress
            value=${populateProgress.steps_done}
            max=${populateProgress.total_steps}
          />
          <div class="populate-progress-details">
            <span>
              ${populateProgress.is_complete
                ? html`<span class="populate-complete">All steps finished</span>`
                : html`Generating: ${populateProgress.current_step}`}
            </span>
            <span>${populateProgress.steps_done} / ${populateProgress.total_steps} steps</span>
            <span>${populateProgress.rows_written.toLocaleString()} rows written</span>
          </div>
        </article>
      `}

      ${dynamoProgress && html`
        <article class="dynamo-progress">
          <header>${dynamoProgress.is_complete
            ? html`${dynamoProgress.operation} Complete`
            : dynamoProgress.operation}</header>
          <progress
            value=${dynamoProgress.items_processed}
            max=${dynamoProgress.total_items || undefined}
          />
          <div class="dynamo-progress-details">
            <span>
              ${dynamoProgress.is_complete
                ? html`<span class="dynamo-complete">All tables processed</span>`
                : html`${dynamoProgress.current_table}`}
            </span>
            <span>${dynamoProgress.tables_done} / ${dynamoProgress.total_tables} tables</span>
            <span>${dynamoProgress.items_processed.toLocaleString()}${
              dynamoProgress.total_items > 0
                ? html` / ${dynamoProgress.total_items.toLocaleString()}`
                : ''
            } items</span>
          </div>
        </article>
      `}

      ${timelapse && html`<${Heatmap} timelapse=${timelapse} />`}

      <${PopulateForm}
        config=${staticConfig}
        onChange=${setStaticConfig}
        disabled=${running}
        onSubmit=${handlePopulateStatic}
        loading=${actionLoading === 'populate-static'}
        hasStaticData=${hasStaticData}
      />

      <${DynamicPopulateForm}
        config=${dynamicConfig}
        onChange=${setDynamicConfig}
        disabled=${running}
        onSubmit=${handlePopulateDynamic}
        loading=${actionLoading === 'populate-dynamic'}
        populatedDates=${populatedDates}
        hasStaticData=${hasStaticData}
      />

      <${DateRangeForm}
        config=${dateRangeConfig}
        onChange=${setDateRangeConfig}
        disabled=${running}
        onSubmit=${handleDateRangeSimulate}
        loading=${actionLoading === 'date-range'}
      />

      <div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem;">
        <button class="outline"
                onclick=${handleRefreshDbCounts}
                disabled=${dbCountsLoading}
                aria-busy=${dbCountsLoading}
                style="width: auto; margin: 0;">
          Refresh from DB
        </button>
        ${dbCounts && html`
          <small style="color: var(--pico-muted-color);">Live counts from database</small>
        `}
      </div>
      <div class="dashboard-grid">
        <${CountTable} title="Aurora DSQL" counts=${auroraCounts} />
        <${CountTable} title="DynamoDB" counts=${dynamoCounts} />
      </div>

      <${ConfirmModal}
        open=${confirmAction !== null}
        title=${confirmAction?.title || ''}
        message=${confirmAction?.message || ''}
        onConfirm=${confirmReset}
        onCancel=${() => setConfirmAction(null)}
      />
    </main>
  `;
}

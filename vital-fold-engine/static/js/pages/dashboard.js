import { h } from 'https://esm.sh/preact@10';
import { useState, useEffect, useRef } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import * as api from '../api.js';
import { StatusBadge } from '../components/status-badge.js';
import { CountTable } from '../components/count-table.js';
import { PopulateForm } from '../components/populate-form.js';
import { ConfirmModal } from '../components/confirm-modal.js';
import { Heatmap } from '../components/heatmap.js';

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
  const [config, setConfig] = useState({
    providers: 50,
    patients: 50000,
    plans_per_company: 3,
    appointments_per_patient: 2,
    records_per_appointment: 1,
  });

  const intervalRef = useRef(null);
  const heatmapRef = useRef(null);

  async function fetchStatus() {
    try {
      const data = await api.get('/simulate/status');
      setStatus(data);
      setRunning(data.running);
    } catch (err) {
      // 401 handled by api.js — other errors shown briefly
      if (err.message !== 'Session expired') {
        setError(err.message);
      }
    }
  }

  // Poll for status
  useEffect(() => {
    fetchStatus();

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
      } catch (_) { /* ignore — status polling handles auth errors */ }
    }

    fetchHeatmap();
    heatmapRef.current = setInterval(fetchHeatmap, 2000);
    return () => clearInterval(heatmapRef.current);
  }, [running]);

  async function handleAction(action, path, body) {
    setError('');
    setActionLoading(action);
    try {
      await api.post(path, body);
      await fetchStatus();
    } catch (err) {
      setError(err.message);
    } finally {
      setActionLoading('');
    }
  }

  function handlePopulate() {
    handleAction('populate', '/populate', config);
  }

  function handleSimulate() {
    handleAction('simulate', '/simulate');
  }

  function handleStop() {
    handleAction('stop', '/simulate/stop');
  }

  function handleTimelapse() {
    handleAction('timelapse', '/simulate/timelapse', { window_interval_secs: 5 });
  }

  function requestReset(type) {
    if (type === 'aurora') {
      setConfirmAction({
        title: 'Reset Aurora DSQL Data',
        message: 'This will permanently delete all generated Aurora DSQL data. This cannot be undone.',
        action: () => handleAction('reset', '/simulate/reset'),
      });
    } else {
      setConfirmAction({
        title: 'Reset DynamoDB Data',
        message: 'This will permanently delete all DynamoDB patient visit and vitals data. This cannot be undone.',
        action: () => handleAction('reset-dynamo', '/simulate/reset-dynamo'),
      });
    }
  }

  async function confirmReset() {
    if (confirmAction) {
      await confirmAction.action();
      setConfirmAction(null);
    }
  }

  // Build count arrays from flattened status response
  const auroraCounts = AURORA_FIELDS.map(f => ({
    label: f.label,
    value: status ? status[f.key] : 0,
  }));

  const dynamoCounts = DYNAMO_FIELDS.map(f => ({
    label: f.label,
    value: status ? status[f.key] : 0,
  }));

  return html`
    <main class="container">
      ${error && html`<p class="error-msg">${error}</p>`}

      <div class="dashboard-grid">
        <${StatusBadge} running=${running} lastRun=${status?.last_run} />

        <article>
          <header>Controls</header>
          <div class="controls-group">
            <button onclick=${handlePopulate}
                    disabled=${running || actionLoading === 'populate'}
                    aria-busy=${actionLoading === 'populate'}>
              Populate
            </button>
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
            <button class="outline btn-danger" onclick=${() => requestReset('dynamo')}
                    disabled=${running}>
              Reset DynamoDB
            </button>
          </div>
        </article>
      </div>

      ${timelapse && html`<${Heatmap} timelapse=${timelapse} />`}

      <${PopulateForm} config=${config} onChange=${setConfig} disabled=${running} />

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

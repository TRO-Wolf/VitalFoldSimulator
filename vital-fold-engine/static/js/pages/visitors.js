import { h } from 'https://esm.sh/preact@10';
import { useState, useEffect, useRef } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';
import * as api from '../api.js';
import { Heatmap } from '../components/heatmap.js';

const html = htm.bind(h);

export function VisitorsPage() {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState('');
  const [actionLoading, setActionLoading] = useState('');
  const [timelapse, setTimelapse] = useState(null);
  const [visitors, setVisitors] = useState(null);

  const heatmapRef = useRef(null);
  const statusRef = useRef(null);

  // Poll simulation status
  useEffect(() => {
    async function fetchStatus() {
      try {
        const data = await api.get('/simulate/status');
        setRunning(data.running);
      } catch (_) {}
    }

    fetchStatus();
    statusRef.current = setInterval(fetchStatus, running ? 2000 : 10000);
    return () => clearInterval(statusRef.current);
  }, [running]);

  // Poll heatmap data when running
  useEffect(() => {
    if (!running) return;

    async function fetchHeatmap() {
      try {
        const data = await api.get('/simulate/heatmap');
        if (data && data.clinics) setTimelapse(data);
      } catch (_) {}
    }

    fetchHeatmap();
    heatmapRef.current = setInterval(fetchHeatmap, 2000);
    return () => clearInterval(heatmapRef.current);
  }, [running]);

  // Fetch visitor list when not running
  useEffect(() => {
    if (running) return;

    async function fetchVisitors() {
      try {
        const data = await api.get('/simulate/visitors');
        if (data && data.clinics) setVisitors(data);
      } catch (_) {}
    }

    fetchVisitors();
  }, [running]);

  async function handleStartReplay() {
    setError('');
    setActionLoading('start');
    try {
      await api.post('/simulate/replay', { window_interval_secs: 5 });
      setTimelapse(null);
      setVisitors(null);
      setRunning(true);
    } catch (err) {
      setError(err.message);
    } finally {
      setActionLoading('');
    }
  }

  async function handleResetReplay() {
    setError('');
    setActionLoading('reset');
    try {
      await api.post('/simulate/replay-reset');
      setTimelapse(null);
      setVisitors(null);
    } catch (err) {
      setError(err.message);
    } finally {
      setActionLoading('');
    }
  }

  const hasVisitors = visitors && visitors.clinics && visitors.clinics.length > 0;

  return html`
    <main class="container">
      ${error && html`<p class="error-msg">${error}</p>`}

      <article>
        <header>Simulation Controls</header>
        <div class="controls-group">
          <button onclick=${handleStartReplay}
                  disabled=${running || actionLoading === 'start'}
                  aria-busy=${actionLoading === 'start'}>
            Start Simulation
          </button>
          <button class="secondary" onclick=${handleResetReplay}
                  disabled=${running || actionLoading === 'reset'}
                  aria-busy=${actionLoading === 'reset'}>
            Reset Simulation
          </button>
        </div>
      </article>

      ${timelapse && html`<${Heatmap} timelapse=${timelapse} />`}

      ${hasVisitors && html`
        <article>
          <header>Today's Visitors — ${visitors.date}</header>
          <div class="visitor-grid">
            ${visitors.clinics.map(clinic => html`
              <div class="visitor-clinic-card" key=${clinic.clinic_name}>
                <h4 class="visitor-clinic-name">${clinic.clinic_name}</h4>
                <small class="visitor-clinic-location">${clinic.city}, ${clinic.state}</small>
                <ul class="visitor-list">
                  ${clinic.visitors.map((v, i) => html`
                    <li key=${i}>
                      <span class="visitor-name">${v.first_name} ${v.last_name}</span>
                      <span class="visitor-hour">${v.hour}:00</span>
                    </li>
                  `)}
                </ul>
                <small class="visitor-count">${clinic.visitors.length} visitor${clinic.visitors.length !== 1 ? 's' : ''}</small>
              </div>
            `)}
          </div>
        </article>
      `}

      ${!running && !hasVisitors && !timelapse && html`
        <article>
          <p>No simulation data yet. An administrator must populate the database from the Dashboard before running the simulation.</p>
        </article>
      `}
    </main>
  `;
}

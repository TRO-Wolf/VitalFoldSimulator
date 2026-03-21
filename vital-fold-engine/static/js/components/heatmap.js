import { h } from 'https://esm.sh/preact@10';
import { useRef, useEffect } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

// Lat/lng for the 7 unique clinic cities
const CLINIC_COORDS = {
  'Charlotte,NC':    { lat: 35.2271, lng: -80.8431 },
  'Asheville,NC':    { lat: 35.5951, lng: -82.5515 },
  'Atlanta,GA':      { lat: 33.7490, lng: -84.3880 },
  'Tallahassee,FL':  { lat: 30.4383, lng: -84.2807 },
  'Miami,FL':        { lat: 25.7617, lng: -80.1918 },
  'Orlando,FL':      { lat: 28.5383, lng: -81.3792 },
  'Jacksonville,FL': { lat: 30.3322, lng: -81.6557 },
};

// Simplified SE US outline (lat/lng points tracing FL/GA/NC/SC coastline and borders)
const SE_US_OUTLINE = [
  // NC northern border (west to east)
  { lat: 36.0, lng: -84.3 },
  { lat: 36.0, lng: -82.5 },
  { lat: 36.2, lng: -81.0 },
  { lat: 36.0, lng: -79.5 },
  // NC coast
  { lat: 35.2, lng: -78.5 },
  { lat: 34.2, lng: -77.8 },
  // SC/GA coast
  { lat: 33.0, lng: -79.2 },
  { lat: 32.0, lng: -80.8 },
  { lat: 31.5, lng: -81.1 },
  // GA/FL coast
  { lat: 30.7, lng: -81.4 },
  // FL Atlantic coast
  { lat: 29.8, lng: -81.3 },
  { lat: 28.5, lng: -80.6 },
  { lat: 27.5, lng: -80.2 },
  { lat: 26.5, lng: -80.1 },
  { lat: 25.5, lng: -80.2 },
  // FL tip
  { lat: 25.1, lng: -80.8 },
  { lat: 25.0, lng: -81.2 },
  // FL Gulf coast
  { lat: 25.8, lng: -81.8 },
  { lat: 26.6, lng: -82.2 },
  { lat: 28.0, lng: -82.8 },
  { lat: 29.0, lng: -83.2 },
  { lat: 29.8, lng: -84.0 },
  // FL panhandle
  { lat: 30.0, lng: -84.4 },
  { lat: 30.4, lng: -85.5 },
  { lat: 30.5, lng: -87.5 },
  // AL/GA western border back up
  { lat: 31.0, lng: -85.0 },
  { lat: 33.0, lng: -85.0 },
  { lat: 35.0, lng: -85.0 },
  // Back to NC
  { lat: 36.0, lng: -84.3 },
];

// Bounding box for projection
const BOUNDS = { minLat: 24.5, maxLat: 36.5, minLng: -88.0, maxLng: -77.0 };

function project(lat, lng, w, h) {
  const pad = 30;
  const x = pad + ((lng - BOUNDS.minLng) / (BOUNDS.maxLng - BOUNDS.minLng)) * (w - pad * 2);
  const y = pad + ((BOUNDS.maxLat - lat) / (BOUNDS.maxLat - BOUNDS.minLat)) * (h - pad * 2);
  return { x, y };
}

// HSL interpolation: blue (low) → orange (high)
function activityColor(active, maxActive) {
  if (active === 0) return 'rgba(100, 120, 140, 0.5)';
  const t = Math.min(active / Math.max(maxActive, 1), 1);
  const hue = 210 - t * 180; // 210 (blue) → 30 (orange)
  const sat = 60 + t * 30;
  const lit = 50 + t * 10;
  return `hsl(${hue}, ${sat}%, ${lit}%)`;
}

function drawMap(canvas, timelapse) {
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const h = canvas.height;

  // Clear
  ctx.fillStyle = '#1a1a2e';
  ctx.fillRect(0, 0, w, h);

  // Draw SE US outline
  ctx.beginPath();
  SE_US_OUTLINE.forEach((p, i) => {
    const { x, y } = project(p.lat, p.lng, w, h);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.fillStyle = 'rgba(40, 50, 70, 0.6)';
  ctx.fill();
  ctx.strokeStyle = 'rgba(100, 130, 160, 0.4)';
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // State borders (approximate)
  // FL/GA border ~30.7 lat
  drawBorderLine(ctx, w, h, 30.7, -87.5, 30.7, -81.4);
  // GA/NC border ~35.0 lat (simplified)
  drawBorderLine(ctx, w, h, 35.0, -85.0, 35.2, -78.5);

  if (!timelapse || !timelapse.clinics) return;

  // Aggregate by city (some cities have 2 clinics)
  const cityMap = {};
  timelapse.clinics.forEach(c => {
    const key = `${c.city},${c.state}`;
    if (!cityMap[key]) cityMap[key] = 0;
    cityMap[key] += c.active_patients;
  });

  const maxActive = Math.max(1, ...Object.values(cityMap));

  // Draw clinic dots
  Object.entries(cityMap).forEach(([key, active]) => {
    const coords = CLINIC_COORDS[key];
    if (!coords) return;

    const { x, y } = project(coords.lat, coords.lng, w, h);
    const radius = Math.max(6, Math.sqrt(active) * 3);
    const color = activityColor(active, maxActive);

    // Glow
    if (active > 0) {
      ctx.save();
      ctx.shadowColor = color;
      ctx.shadowBlur = radius * 1.5;
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
      ctx.restore();
    }

    // Dot
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,255,255,0.3)';
    ctx.lineWidth = 1;
    ctx.stroke();

    // Label
    const city = key.split(',')[0];
    ctx.fillStyle = 'rgba(220, 230, 240, 0.9)';
    ctx.font = '11px system-ui, sans-serif';
    ctx.textAlign = 'left';
    ctx.fillText(city, x + radius + 4, y + 4);

    // Count
    if (active > 0) {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
      ctx.font = 'bold 10px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(active.toLocaleString(), x, y - radius - 4);
    }
  });

  // Day/hour label
  if (timelapse.simulation_day) {
    const hourStr = timelapse.sim_hour != null
      ? `${timelapse.sim_hour}:00`
      : '';
    const label = timelapse.total_days === 1
      ? `${timelapse.simulation_day}  ${hourStr}`
      : `Day ${timelapse.day_number} of ${timelapse.total_days}  ${timelapse.simulation_day}  ${hourStr}`;
    ctx.fillStyle = 'rgba(200, 210, 220, 0.8)';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(label, w / 2, h - 10);
  }
}

function drawBorderLine(ctx, w, h, lat1, lng1, lat2, lng2) {
  const p1 = project(lat1, lng1, w, h);
  const p2 = project(lat2, lng2, w, h);
  ctx.beginPath();
  ctx.moveTo(p1.x, p1.y);
  ctx.lineTo(p2.x, p2.y);
  ctx.strokeStyle = 'rgba(100, 130, 160, 0.25)';
  ctx.lineWidth = 1;
  ctx.setLineDash([4, 4]);
  ctx.stroke();
  ctx.setLineDash([]);
}

export function Heatmap({ timelapse }) {
  const canvasRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Set actual pixel dimensions (CSS controls display size)
    canvas.width = 500;
    canvas.height = 500;

    drawMap(canvas, timelapse);
  }, [timelapse]);

  const progress = timelapse
    ? timelapse.total_days === 1
      ? ((timelapse.sim_hour - 9) / 8) * 100
      : (timelapse.day_number / timelapse.total_days) * 100
    : 0;

  const isComplete = timelapse?.is_complete;

  return html`
    <article>
      <header>Clinic Heatmap</header>
      <div class="heatmap-container">
        <canvas ref=${canvasRef} class="heatmap-canvas" />
        <div class="heatmap-progress">
          <div class="heatmap-progress-fill" style=${{ width: `${progress}%` }} />
        </div>
        <div class="heatmap-day-label">
          ${isComplete
            ? 'Complete'
            : timelapse
              ? timelapse.total_days === 1
                ? `${timelapse.sim_hour}:00 — ${Math.round(progress)}%`
                : `${Math.round(progress)}% complete`
              : 'Waiting...'}
        </div>
      </div>
    </article>
  `;
}

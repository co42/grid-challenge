export function formatDistance(km) {
  if (!km) return '—';
  return km < 10 ? km.toFixed(1) + ' km' : Math.round(km) + ' km';
}

export function formatDuration(secs) {
  if (!secs) return null;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h${String(m).padStart(2, '0')}` : `${m}min`;
}

export function formatPace(km, secs) {
  if (!km || !secs || km < 0.01) return null;
  const minPerKm = secs / 60 / km;
  const m = Math.floor(minPerKm);
  const s = Math.round((minPerKm - m) * 60);
  return `${m}:${String(s).padStart(2, '0')}/km`;
}

export function formatDate(iso) {
  if (!iso) return null;
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  } catch {
    return null;
  }
}

export function formatElevation(m) {
  if (!m) return '—';
  return Math.round(m) + 'm';
}

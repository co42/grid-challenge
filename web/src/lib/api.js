const BASE = '/api';

async function request(method, path, body) {
  const opts = {
    method,
    credentials: 'same-origin',
    headers: {},
  };

  if (body && !(body instanceof FormData)) {
    opts.headers['Content-Type'] = 'application/json';
    opts.body = JSON.stringify(body);
  } else if (body) {
    opts.body = body;
  }

  const res = await fetch(`${BASE}${path}`, opts);

  if (res.status === 401) {
    throw new Error('Unauthorized');
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || res.statusText);
  }

  return res.json();
}

export const api = {
  get: (path) => request('GET', path),
  post: (path, body) => request('POST', path, body),
  delete: (path) => request('DELETE', path),

  // Auth
  register: (email, password) => request('POST', '/auth/register', { email, password }),
  login: (email, password) => request('POST', '/auth/login', { email, password }),
  logout: () => request('POST', '/auth/logout'),
  me: () => request('GET', '/auth/me'),

  // Challenges
  listChallenges: () => request('GET', '/challenges'),
  getChallenge: (id) => request('GET', `/challenges/${id}`),
  createChallenge: (data) => request('POST', '/challenges', data),
  updateChallenge: (id, data) => request('PATCH', `/challenges/${id}`, data),
  deleteChallenge: (id) => request('DELETE', `/challenges/${id}`),
  refreshChallenge: (id) => request('POST', `/challenges/${id}/refresh`),

  // GPX
  listGpx: () => request('GET', '/gpx'),
  uploadGpx: (file) => {
    const fd = new FormData();
    fd.append('file', file);
    return request('POST', '/gpx/upload', fd);
  },
  deleteGpx: (id) => request('DELETE', `/gpx/${id}`),
  deleteAllGpx: () => request('DELETE', '/gpx'),

  // Share
  getShared: (token) => request('GET', `/share/${token}`),

  // Preview
  previewGrid: (polygon, grid_size) => request('POST', '/preview/grid', { polygon, grid_size }),

  // Strava
  stravaAuthorize: () => request('GET', '/strava/authorize'),
  stravaStatus: () => request('GET', '/strava/status'),
  stravaSyncStream: async (onProgress) => {
    const res = await fetch(`${BASE}/strava/sync`, {
      method: 'POST',
      credentials: 'same-origin',
    });
    if (res.status === 401) throw new Error('Unauthorized');
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error || res.statusText);
    }
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let result = { imported: 0 };
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop();
      for (const line of lines) {
        if (!line.trim()) continue;
        const event = JSON.parse(line);
        if (event.type === 'done') result = event;
        else onProgress?.(event);
      }
    }
    return result;
  },
  stravaDisconnect: (deleteFiles = false) =>
    request('POST', `/strava/disconnect?delete_files=${deleteFiles}`),
};

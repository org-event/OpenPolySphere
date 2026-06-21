/** HTTP helpers for the Rust backend. */

export async function sendCmd(cmd) {
  const resp = await fetch('/cmd', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cmd }),
  });
  return resp.json();
}

export async function fetchJson(url, options) {
  const resp = await fetch(url, options);
  const data = await resp.json();
  if (!resp.ok) {
    const err = new Error(data.error || resp.statusText);
    err.status = resp.status;
    err.data = data;
    throw err;
  }
  return data;
}

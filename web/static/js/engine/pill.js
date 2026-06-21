export function setEnginePill(pillState, text) {
  const pill = document.getElementById('engine-pill');
  if (!pill) return;
  pill.className = 'engine-pill' + (pillState === 'running' ? '' : ' ' + pillState);
  const label = document.getElementById('engine-label');
  if (label) label.textContent = text;
}

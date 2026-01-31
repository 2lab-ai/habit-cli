function stableClone(value) {
  if (value === null) return null;
  if (Array.isArray(value)) return value.map(stableClone);
  if (typeof value === 'object') {
    const keys = Object.keys(value).sort();
    /** @type {any} */
    const out = {};
    for (const k of keys) out[k] = stableClone(value[k]);
    return out;
  }
  return value;
}

function stableStringify(value) {
  return JSON.stringify(stableClone(value), null, 2);
}

module.exports = { stableStringify };

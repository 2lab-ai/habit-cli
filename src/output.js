const ANSI = {
  reset: '\u001b[0m',
  green: '\u001b[32m',
  gray: '\u001b[90m'
};

function makeStyler({ colorEnabled }) {
  function wrap(code, s) {
    if (!colorEnabled) return s;
    return code + s + ANSI.reset;
  }

  return {
    green: (s) => wrap(ANSI.green, s),
    gray: (s) => wrap(ANSI.gray, s)
  };
}

function padRight(s, width) {
  const str = String(s);
  if (str.length >= width) return str;
  return str + ' '.repeat(width - str.length);
}

function renderSimpleTable(rows, columns) {
  // columns: [{ key, header, value(row) }]
  const widths = columns.map((c) => c.header.length);
  for (const row of rows) {
    columns.forEach((c, i) => {
      const v = String(c.value(row));
      widths[i] = Math.max(widths[i], v.length);
    });
  }

  const header = columns.map((c, i) => padRight(c.header, widths[i])).join('  ');
  const body = rows.map((row) => columns.map((c, i) => padRight(String(c.value(row)), widths[i])).join('  ')).join('\n');
  return body ? `${header}\n${body}` : header;
}

module.exports = {
  makeStyler,
  renderSimpleTable
};

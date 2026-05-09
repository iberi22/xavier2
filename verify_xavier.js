const http = require('http');

function api(BASE, TOKEN, path, payload, method) {
  return new Promise((resolve, reject) => {
    const url = new URL(path, BASE);
    const data = payload ? JSON.stringify(payload) : null;
    const options = {
      hostname: url.hostname, port: url.port, path: url.pathname,
      method: method || (payload ? 'POST' : 'GET'),
      headers: {'Content-Type': 'application/json', 'X-Xavier-Token': TOKEN}
    };
    if (data) options.headers['Content-Length'] = Buffer.byteLength(data);
    const req = http.request(options, res => {
      let body = '';
      res.on('data', c => body += c);
      res.on('end', () => { try { resolve(JSON.parse(body)); } catch { resolve(body); } });
    });
    req.on('error', reject);
    if (data) req.write(data);
    req.end();
  });
}

async function main() {
  console.log('=== Verifying Xavier memories ===\n');

  const queries = ['*', 'task', 'repo', 'decision', 'session', 'xavier', 'memory', 'openclaw'];

  for (const q of queries) {
    const r = await api('http://localhost:8006', 'dev-token', '/memory/search', {query: q, limit: 20});
    const count = r.results ? r.results.length : 0;
    console.log('Query "' + q + '": ' + count + ' results');
    if (r.results && r.results.length > 0) {
      for (const m of r.results) {
        console.log('  -> ' + m.path + ' (score: ' + (m.score||'n/a') + ')');
      }
    }
  }

  console.log('\n--- /memory/manage ---');
  try {
    const m = await api('http://localhost:8006', 'dev-token', '/memory/manage', {action: 'stats'});
    console.log(JSON.stringify(m, null, 2));
  } catch(e) { console.log('Error: ' + e.message); }
}

main().catch(e => console.error(e.message));
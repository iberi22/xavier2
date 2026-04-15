#!/usr/bin/env node
const http = require('http');

const BASE_URL = 'http://localhost:8003';
const TOKEN = 'dev-token';

function api(path, payload, method) {
  return new Promise((resolve, reject) => {
    const url = new URL(path, BASE_URL);
    const data = payload ? JSON.stringify(payload) : null;
    const options = {
      hostname: url.hostname,
      port: url.port,
      path: url.pathname,
      method: method || (payload ? 'POST' : 'GET'),
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier2-Token': TOKEN,
      },
    };
    if (data) options.headers['Content-Length'] = Buffer.byteLength(data);
    const req = http.request(options, (res) => {
      let body = '';
      res.on('data', c => body += c);
      res.on('end', () => {
        try { resolve(JSON.parse(body)); }
        catch { resolve(body); }
      });
    });
    req.on('error', reject);
    if (data) req.write(data);
    req.end();
  });
}

async function waitHealth() {
  for (let i = 0; i < 60; i++) {
    try {
      const r = await api('/health', null, 'GET');
      if (r.status === 'ok') return;
    } catch {}
    await new Promise(s => setTimeout(s, 1000));
  }
  throw new Error('Xavier2 not healthy');
}

async function loadDocs(docs) {
  await api('/memory/reset', {});
  const t0 = Date.now();
  for (const d of docs) {
    await api('/memory/add', {
      path: d.path, content: d.content, metadata: d.metadata || {},
      kind: d.kind, evidence_kind: d.evidence_kind,
      namespace: d.namespace, provenance: d.provenance,
    });
  }
  return (Date.now() - t0) / docs.length;
}

async function doSearch(query, filters) {
  const p = { query, limit: 5 };
  if (filters) p.filters = filters;
  const t0 = Date.now();
  const r = await api('/memory/search', p);
  return { results: r.results || [], ms: Date.now() - t0 };
}

async function doQuery(query, filters, system3) {
  const p = { query, limit: 5, system3_mode: system3 || 'disabled' };
  if (filters) p.filters = filters;
  const t0 = Date.now();
  const r = await api('/memory/query', p);
  return { response: r.response || '', ms: Date.now() - t0 };
}

async function doAgents(query, filters) {
  const p = { query, limit: 5 };
  if (filters) p.filters = filters;
  const t0 = Date.now();
  const r = await api('/agents/run', p);
  return { response: r.response || '', ms: Date.now() - t0 };
}

async function run() {
  console.log('Waiting for Xavier2...');
  await waitHealth();
  console.log('Xavier2 is healthy.');

  const fs = require('fs');
  const dataset = JSON.parse(fs.readFileSync(
    'E:\\scripts-python\\xavier2\\scripts\\benchmarks\\datasets\\internal_swal_openclaw_memory.json', 'utf-8'));

  const docs = dataset.documents;
  const cases = dataset.cases;

  console.log(`\nLoading ${docs.length} documents...`);
  const loadMs = await loadDocs(docs);
  console.log(`  Load time: ${loadMs.toFixed(1)}ms per document`);

  const results = [];
  const sLat = [], qLat = [];
  let correct = 0;

  console.log(`\nRunning ${cases.length} benchmark cases...`);
  for (const c of cases) {
    let hit = false;
    let latency = 0;

    if (c.endpoint === 'search') {
      const { results: res, ms } = await doSearch(c.query, c.filters);
      latency = ms;
      sLat.push(ms);
      const top = res[0] ? res[0].path : null;
      hit = top === c.expected_path;
      results.push({ id: c.id, endpoint: 'search', query: c.query, expected: c.expected_path, actual: top, hit, latency_ms: +ms.toFixed(1) });
    } else if (c.endpoint === 'query') {
      const { response, ms } = await doQuery(c.query, c.filters, c.system3_mode);
      latency = ms;
      qLat.push(ms);
      hit = c.expected_substring && response.toLowerCase().includes(c.expected_substring.toLowerCase());
      results.push({ id: c.id, endpoint: 'query', query: c.query, expected: c.expected_substring, actual: response.substring(0, 200), hit, latency_ms: +ms.toFixed(1) });
    } else if (c.endpoint === 'agents_run') {
      const { response, ms } = await doAgents(c.query, c.filters);
      latency = ms;
      qLat.push(ms);
      hit = c.expected_substring && response.toLowerCase().includes(c.expected_substring.toLowerCase());
      results.push({ id: c.id, endpoint: 'agents_run', query: c.query, expected: c.expected_substring, actual: response.substring(0, 200), hit, latency_ms: +ms.toFixed(1) });
    }

    if (hit) correct++;
    console.log(`  [${hit ? 'PASS' : 'FAIL'}] ${c.id} (${latency.toFixed(1)}ms)`);
  }

  const acc = (correct / cases.length * 100).toFixed(1);
  const avgS = sLat.length ? (sLat.reduce((a, b) => a + b, 0) / sLat.length).toFixed(1) : 0;
  const avgQ = qLat.length ? (qLat.reduce((a, b) => a + b, 0) / qLat.length).toFixed(1) : 0;
  const build = await api('/build', null, 'GET');

  const summary = {
    timestamp: new Date().toISOString(),
    backend: build.memory_store.backend,
    version: build.version,
    total_cases: cases.length,
    passed: correct,
    accuracy_pct: +acc,
    avg_search_ms: +avgS,
    avg_query_ms: +avgQ,
    load_ms_per_doc: +loadMs.toFixed(1),
    search_cases: sLat.length,
    query_cases: qLat.length,
  };

  console.log(`\n${'='.repeat(50)}`);
  console.log(`RESULTS -- ${summary.backend} backend (v${summary.version})`);
  console.log(`${'='.repeat(50)}`);
  console.log(`Accuracy:   ${acc}%  (${correct}/${cases.length})`);
  console.log(`Avg search: ${avgS}ms`);
  console.log(`Avg query:  ${avgQ}ms`);
  console.log(`Load/doc:   ${loadMs.toFixed(1)}ms`);

  const outDir = 'benchmark-results/real-memory-benchmark';
  require('fs').mkdirSync(outDir, { recursive: true });
  require('fs').writeFileSync(`${outDir}/summary.json`, JSON.stringify(summary, null, 2));
  require('fs').writeFileSync(`${outDir}/records.json`, JSON.stringify(results, null, 2));
  console.log(`\nSaved: ${outDir}/summary.json and records.json`);
}

run().catch(e => { console.error(e); process.exit(1); });

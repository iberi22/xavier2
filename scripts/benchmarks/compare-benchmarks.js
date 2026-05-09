#!/usr/bin/env node
/**
 * Xavier Benchmark Comparator
 * Compara resultados de benchmarks entre backends usando timestamps
 * Uso: node compare-benchmarks.js [directorio-resultados]
 */

const fs = require('fs');
const path = require('path');

const RESULTS_DIR = path.join(__dirname, '..', 'benchmark-results');

// ANSI colors
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const BOLD = '\x1b[1m';
const RESET = '\x1b[0m';

function loadJson(filepath) {
  try { return JSON.parse(fs.readFileSync(filepath, 'utf-8')); }
  catch { return null; }
}

function parseBackendFromDir(dirname) {
  // Formato: file-2026-04-13, vec-2026-04-13, surreal-2026-04-13
  const parts = dirname.split('-');
  return { backend: parts[0], date: parts.slice(1).join('-') };
}

function findBenchmarks() {
  const dirs = fs.readdirSync(RESULTS_DIR).filter(d => {
    return fs.statSync(path.join(RESULTS_DIR, d)).isDirectory();
  });

  const benchmarks = [];
  for (const dir of dirs) {
    const summaryPath = path.join(RESULTS_DIR, dir, 'summary.json');
    const recordsPath = path.join(RESULTS_DIR, dir, 'records.json');
    const summary = loadJson(summaryPath);
    const records = loadJson(recordsPath);
    if (summary) {
      benchmarks.push({
        dir,
        summary,
        records: records || [],
        ...parseBackendFromDir(dir)
      });
    }
  }
  return benchmarks.sort((a, b) => a.dir.localeCompare(b.dir));
}

function printTable(rows, headers) {
  const colWidths = headers.map((h, i) => {
    const max = Math.max(h.length, ...rows.map(r => String(r[i] || '').length));
    return max + 2;
  });

  const headerLine = headers.map((h, i) => h.padEnd(colWidths[i]).padStart(colWidths[i] - h.length/2 | 0)).join(' | ');
  console.log(BOLD + headerLine + RESET);
  console.log(colWidths.map(w => '-'.repeat(w)).join('-+-'));
  rows.forEach(row => {
    const line = row.map((cell, i) => {
      const s = String(cell || '');
      return s.padEnd(colWidths[i]).padStart(colWidths[i] - s.length/2 | 0);
    }).join(' | ');
    console.log(line);
  });
}

function compareTwo(a, b) {
  const diff = {};
  const metrics = ['accuracy_pct', 'avg_search_ms', 'avg_query_ms', 'load_ms_per_doc', 'total_cases', 'passed'];

  console.log(`\n${BOLD}Comparacion: ${a.backend} vs ${b.backend}${RESET}`);
  console.log(`${'─'.repeat(50)}`);

  for (const m of metrics) {
    const va = a.summary[m];
    const vb = b.summary[m];
    if (va === undefined || vb === undefined) continue;

    const diff_val = typeof va === 'number' ? vb - va : null;
    const pct = diff_val !== null && va !== 0 ? (diff_val / va * 100).toFixed(1) : null;

    let arrow = '';
    let color = RESET;
    if (diff_val !== null) {
      // Higher is better for accuracy/passed, lower is better for latency
      if (m === 'accuracy_pct' || m === 'passed') {
        if (diff_val > 0) { arrow = ' ▲'; color = GREEN; }
        else if (diff_val < 0) { arrow = ' ▼'; color = RED; }
      } else {
        if (diff_val < 0) { arrow = ' ▼'; color = GREEN; }
        else if (diff_val > 0) { arrow = ' ▼'; color = RED; }
      }
    }

    const valA = typeof va === 'number' ? va.toFixed(1) : va;
    const valB = typeof vb === 'number' ? vb.toFixed(1) : vb;
    const pctStr = pct !== null ? ` (${pct > 0 ? '+' : ''}${pct}%)` : '';

    console.log(`${color}${m.padEnd(20)} ${a.backend.padEnd(10)} → ${b.backend.padEnd(10)} | diff: ${diff_val !== null ? (diff_val > 0 ? '+' : '') + diff_val.toFixed(1) : 'N/A'}${pctStr}${arrow}${RESET}`);
  }
}

function detailedCaseComparison(a, b) {
  console.log(`\n${BOLD}Detalle por caso:${RESET}`);
  const casesA = new Map(a.records.map(r => [r.id, r]));
  const casesB = new Map(b.records.map(r => [r.id, r]));

  const allIds = [...new Set([...casesA.keys(), ...casesB.keys()])];
  allIds.sort();

  console.log(`\n${'ID'.padEnd(25)} ${a.backend.padEnd(10)} ${b.backend.padEnd(10)}`);
  console.log('-'.repeat(50));

  for (const id of allIds) {
    const ra = casesA.get(id);
    const rb = casesB.get(id);
    const hitA = ra ? (ra.hit ? '[PASS]' : '[FAIL]') : '   -  ';
    const hitB = rb ? (rb.hit ? '[PASS]' : '[FAIL]') : '   -  ';
    const diff = (ra && rb) ? ((ra.hit !== rb.hit) ? ' ***' : '    ') : '    ';
    const latA = ra ? `${ra.latency_ms}ms` : '-';
    const latB = rb ? `${rb.latency_ms}ms` : '-';
    console.log(`${id.padEnd(25)} ${hitA} ${latA.padEnd(10)} ${hitB} ${latB.padEnd(10)}${diff}`);
  }
}

function generateReport(benchmarks) {
  console.log(`\n${BOLD}${'='.repeat(70)}`);
  console.log(` XAVIER BENCHMARK REPORT — ${new Date().toISOString().slice(0,10)}`);
  console.log(`${'='.repeat(70)}${RESET}\n`);

  console.log(`${BOLD}Backends encontrados:${RESET}`);
  benchmarks.forEach(b => {
    console.log(`  ${b.backend.padEnd(10)} (${b.dir}) — v${b.summary.version} — ${b.summary.total_cases} casos`);
  });

  if (benchmarks.length < 2) {
    console.log(`\n${YELLOW}Se necesitan al menos 2 benchmarks para comparar.${RESET}`);
    console.log(`Resultados disponibles: ${benchmarks.map(b => b.dir).join(', ')}`);
    return;
  }

  // Pairwise comparisons
  for (let i = 1; i < benchmarks.length; i++) {
    compareTwo(benchmarks[i - 1], benchmarks[i]);
    detailedCaseComparison(benchmarks[i - 1], benchmarks[i]);
  }

  // Summary table
  console.log(`\n${BOLD}Resumen:${RESET}`);
  const rows = benchmarks.map(b => [
    b.backend,
    `${b.summary.accuracy_pct}%`,
    `${b.summary.avg_search_ms}ms`,
    `${b.summary.avg_query_ms}ms`,
    `${b.summary.load_ms_per_doc}ms`,
    `${b.summary.passed}/${b.summary.total_cases}`
  ]);
  printTable(rows, ['Backend', 'Accuracy', 'Avg Search', 'Avg Query', 'Load/Doc', 'Passed']);

  // Export comparison as JSON
  const comparison = {
    timestamp: new Date().toISOString(),
    benchmarks: benchmarks.map(b => ({
      backend: b.backend,
      version: b.summary.version,
      accuracy_pct: b.summary.accuracy_pct,
      avg_search_ms: b.summary.avg_search_ms,
      avg_query_ms: b.summary.avg_query_ms,
      load_ms_per_doc: b.summary.load_ms_per_doc,
      total_cases: b.summary.total_cases,
      passed: b.summary.passed,
    }))
  };

  const outPath = path.join(RESULTS_DIR, `comparison_${Date.now()}.json`);
  fs.writeFileSync(outPath, JSON.stringify(comparison, null, 2));
  console.log(`\n${CYAN}Reporte guardado: ${outPath}${RESET}`);
}

function main() {
  console.log('Cargando benchmarks de:', RESULTS_DIR);
  const benchmarks = findBenchmarks();
  if (benchmarks.length === 0) {
    console.log('No se encontraron resultados en', RESULTS_DIR);
    console.log('Ejecutar primero: node bench_real.js');
    return;
  }
  generateReport(benchmarks);
}

main();
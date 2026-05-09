#!/usr/bin/env node
/**
 * SWAL Xavier - Code Indexing Script
 * Indexes SWAL code repositories for code search
 *
 * Usage: node index-swal-code.js
 *
 * IMPORTANT: Xavier runs in Docker and paths inside the container are Unix-style.
 * The SWAL repos are mounted at /mnt/swal/ inside the container.
 */

const XAVIER_URL = process.env.XAVIER_URL || 'http://localhost:8003';

function getRequiredXavierToken() {
  const token = process.env.XAVIER_TOKEN || process.env.XAVIER_API_KEY || process.env.XAVIER_TOKEN;
  if (!token) {
    throw new Error('Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.');
  }
  return token;
}

const XAVIER_TOKEN = getRequiredXavierToken();

// ===== SWAL REPOS (Unix paths inside Docker container) =====
const SWAL_REPOS_UNIX = [
  '/mnt/swal/xavier/src',
  '/mnt/swal/scripts',
  '/mnt/swal/gestalt-rust/src',
  '/mnt/swal/manteniapp/src',
  '/mnt/swal/synapse-protocol/src',
  '/mnt/swal/synapse-agentic/src',
  '/mnt/swal/synapse-enterprise/src',
];

// Full scan path (indexes ALL repos at once)
const FULL_SCAN_PATH = '/mnt/swal';

// ===== HTTP HELPERS =====
async function codeScan(repoPath) {
  const url = `${XAVIER_URL}/code/scan`;
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier-Token': XAVIER_TOKEN
      },
      body: JSON.stringify({ path: repoPath })
    });

    if (!response.ok) {
      const text = await response.text();
      return { success: false, error: `${response.status}: ${text.substring(0, 200)}` };
    }

    const data = await response.json();
    return {
      success: true,
      indexed_files: data.indexed_files,
      indexed_chunks: data.indexed_chunks
    };
  } catch (e) {
    return { success: false, error: e.message };
  }
}

async function codeStats() {
  const url = `${XAVIER_URL}/code/stats`;
  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: { 'X-Xavier-Token': XAVIER_TOKEN }
    });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
}

async function codeFind(query, limit = 10) {
  const url = `${XAVIER_URL}/code/find`;
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier-Token': XAVIER_TOKEN
      },
      body: JSON.stringify({ query, limit })
    });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
}

// ===== MAIN =====
async function main() {
  console.log('🔍 SWAL Xavier - Code Indexing\n');
  console.log(`   Xavier: ${XAVIER_URL}\n`);

  // Get baseline stats
  const beforeStats = await codeStats();
  console.log('📊 Code Graph Status (before):');
  if (beforeStats) {
    console.log(`   Files: ${beforeStats.total_files}`);
    console.log(`   Symbols: ${beforeStats.total_chunks}\n`);
  } else {
    console.log('   Unable to get stats\n');
  }

  // Strategy: Scan all repos at once via parent directory
  console.log('='.repeat(50));
  console.log('INDEXING STRATEGY: Full SWAL scan (/mnt/swal)');
  console.log('='.repeat(50));
  console.log('\nScanning all SWAL repos at once...\n');

  console.log('⏳ Indexing... (this may take a moment)\n');

  const result = await codeScan(FULL_SCAN_PATH);

  if (result.success && result.indexed_files > 0) {
    console.log(`✅ Scan complete!`);
    console.log(`   Files indexed: ${result.indexed_files}`);
    console.log(`   Symbols extracted: ${result.indexed_chunks}\n`);
  } else if (result.success) {
    console.log(`⚠️  Scan completed but 0 files indexed.`);
    console.log(`   This usually means repos aren't mounted in Docker.\n`);
    console.log(`   Verify mounts with: docker exec xavier ls /mnt/swal/\n`);
  } else {
    console.log(`❌ Scan failed: ${result.error}\n`);
    console.log(`   Make sure Docker volumes are mounted in docker-compose.yml:\n`);
    console.log(`   - E:/scripts-python/xavier:/mnt/swal/xavier:ro`);
    console.log(`   - E:/scripts-python/scripts:/mnt/swal/scripts:ro`);
    console.log(`   etc.\n`);
  }

  // Final stats
  console.log('='.repeat(50));
  console.log('📊 Code Graph Status (after):');
  const afterStats = await codeStats();
  if (afterStats) {
    console.log(`   Files: ${afterStats.total_files}`);
    console.log(`   Symbols: ${afterStats.total_chunks}\n`);
  }

  // Test search
  console.log('🔎 Testing code search...\n');
  const testQueries = ['memory', 'xavier', 'indexer', 'sync'];

  for (const query of testQueries) {
    const results = await codeFind(query, 3);
    if (results && results.results.length > 0) {
      console.log(`   "${query}": ${results.results.length} results`);
      for (const r of results.results.slice(0, 2)) {
        const shortPath = r.path.replace('/mnt/swal/', '');
        console.log(`      - ${r.symbol} (${r.symbol_type}) at ${shortPath}:${r.line}`);
      }
    } else {
      console.log(`   "${query}": no results`);
    }
  }

  console.log('\n✅ Code indexing complete!');
  console.log('\n📝 Usage Examples:');
  console.log('   # Find functions named "memory"');
  console.log('   curl -H "X-Xavier-Token: ${XAVIER_TOKEN}" -X POST http://localhost:8003/code/find \\');
  console.log('     -H "Content-Type: application/json" \\');
  console.log('     -d \'{"query": "memory", "limit": 10}\'');
  console.log('\n   # Find structs only');
  console.log('   curl -H "X-Xavier-Token: ${XAVIER_TOKEN}" -X POST http://localhost:8003/code/find \\');
  console.log('     -H "Content-Type: application/json" \\');
  console.log('     -d \'{"query": "memory", "limit": 10, "kind": "struct"}\'');
  console.log('\n   # Re-index (run this script again)');
  console.log('   node index-swal-code.js');
}

main().catch(console.error);

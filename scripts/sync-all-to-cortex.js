#!/usr/bin/env node
/**
 * SWAL Xavier - Minimal Robust Sync
 * Key principle: finish within 100s, don't hang, skip problematic items
 */

// Limit heap to prevent OS OOM killer (Windows sends SIGKILL when memory pressure detected)
// NOTE: Must be set via NODE_OPTIONS env var, not here. Use the .bat launcher or set system env.
const HEAP_LIMIT = 128; // MB - match NODE_OPTIONS externally set

const XAVIER_URL = process.env.XAVIER_URL || 'http://localhost:8003';
const fs = require('fs');
const path = require('path');
const { setTimeout: delay } = require('timers/promises');
const { execSync } = require('child_process');

function getRequiredXavierToken() {
  const token = process.env.XAVIER_TOKEN || process.env.XAVIER_API_KEY || process.env.XAVIER_TOKEN;
  if (!token) {
    throw new Error('Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.');
  }
  return token;
}

const XAVIER_TOKEN = getRequiredXavierToken();

// Verify heap limit is set (warn if not)
try {
  const heapLimit = parseInt(execSync('node -p "parseInt(process.env.NODE_OPTIONS||0)"', { encoding: 'utf8' }).trim());
  if (heapLimit < 64) {
    console.log('⚠️  WARNING: NODE_OPTIONS=max-old-space-size not set. Memory limits may be insufficient.');
    console.log('⚠️  Run with: NODE_OPTIONS=--max-old-space-size=128 node sync-all-to-xavier.js');
    console.log('⚠️  Or set system env: [System.Environment]::SetEnvironmentVariable("NODE_OPTIONS", "--max-old-space-size=128", "User")');
  }
} catch {}

// ===== HTTP HELPER =====
async function postToXavier(endpoint, payload, timeoutMs = 8000) {
  const url = `${XAVIER_URL}${endpoint}`;
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Xavier-Token': XAVIER_TOKEN
      },
      body: JSON.stringify(payload),
      signal: controller.signal
    });
    clearTimeout(timer);
    return response.ok;
  } catch {
    return false;
  }
}

// ===== CONFIG =====
const AGENT_DIR = 'C:\\Users\\belal\\clawd\\agents\\ventas';
const SESSIONS_DIR = 'C:\\Users\\belal\\.openclaw\\agents\\ventas\\sessions';
const SWAL_DASHBOARD = 'E:\\scripts-python\\SWAL-Operations-Dashboard';

// ===== 1. MEMORY.md =====
async function syncMemoryMd() {
  const filePath = path.join(AGENT_DIR, 'MEMORY.md');
  if (!fs.existsSync(filePath)) return 0;

  const content = fs.readFileSync(filePath, 'utf8');
  const stats = fs.statSync(filePath);
  const sections = content.split(/^## /m).filter(s => s.trim());

  let synced = 0;
  for (const section of sections) {
    const lines = section.split('\n');
    const title = lines[0].replace(/^#+\s*/, '').trim();
    const body = lines.slice(1).join('\n').trim();
    if (body.length < 20) continue;

    const ok = await postToXavier('/memory/add', {
      content: `## ${title}\n\n${body}`,
      path: `openclaw/memory/memory-md/${title.replace(/[^a-zA-Z0-9]/g, '-').toLowerCase()}`,
      metadata: { type: 'memory-md', source: 'openclaw', title, last_modified: stats.mtime.toISOString(), synced_at: new Date().toISOString() }
    });
    if (ok) synced++;
    await delay(300);
  }

  await postToXavier('/memory/add', {
    content: content.substring(0, 5000),
    path: 'openclaw/memory/memory-md/full',
    metadata: { type: 'memory-md-full', source: 'openclaw', title: 'MEMORY.md (Full)', last_modified: stats.mtime.toISOString(), synced_at: new Date().toISOString() }
  });

  return synced;
}

// ===== 2. DAILY MEMORY FILES =====
async function syncDailyFiles() {
  const memoryDir = path.join(AGENT_DIR, 'memory');
  if (!fs.existsSync(memoryDir)) return 0;

  const files = fs.readdirSync(memoryDir).filter(f => f.endsWith('.md')).sort();
  const MAX = Math.min(10, files.length);
  let synced = 0;

  console.log(`   (${MAX}/${files.length} files)`);

  for (let i = 0; i < MAX; i++) {
    const file = files[i];
    const filePath = path.join(memoryDir, file);
    const stats = fs.statSync(filePath);

    if (stats.size > 500 * 1024) {
      console.log(`   ⏭  ${file} (>500KB skip)`);
      continue;
    }

    const content = fs.readFileSync(filePath, 'utf8');
    const date = file.replace('.md', '');
    const title = content.split('\n').find(l => l.startsWith('# '))?.replace('# ', '').trim() || date;

    const ok = await postToXavier('/memory/add', {
      content: content.substring(0, 6000),
      path: `openclaw/memory/daily/${date}`,
      metadata: { type: 'daily-memory', source: 'openclaw', date, title, last_modified: stats.mtime.toISOString(), synced_at: new Date().toISOString() }
    });

    if (ok) synced++;
    else console.log(`   ⚠️  ${file} failed`);

    if ((i + 1) % 5 === 0) console.log(`   ... ${i + 1}/${MAX}`);
    await delay(400);
  }

  return synced;
}

// ===== 3. SESSION LOGS =====
async function syncSessions() {
  if (!fs.existsSync(SESSIONS_DIR)) return 0;

  const allFiles = fs.readdirSync(SESSIONS_DIR)
    .filter(f => f.endsWith('.jsonl') && !f.includes('.reset.') && !f.includes('.deleted.') && !f.includes('.checkpoint.'))
    .sort().slice(-5);

  let synced = 0;

  for (const file of allFiles) {
    const filePath = path.join(SESSIONS_DIR, file);
    const stats = fs.statSync(filePath);

    if (stats.size > 512 * 1024) {
      console.log(`   ⏭  ${file} (>512KB skip)`);
      continue;
    }

    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n').filter(l => l.trim());

    const msgs = [];
    let count = 0;
    for (let j = lines.length - 1; j >= 0 && count < 3; j--) {
      try {
        const record = JSON.parse(lines[j]);
        if (record.type === 'message' && record.message?.role === 'assistant' && record.message?.content) {
          const text = typeof record.message.content === 'string' ? record.message.content : record.message.content[0]?.text || '';
          if (text.length > 20) {
            msgs.unshift(text.substring(0, 400));
            count++;
          }
        }
      } catch {}
    }

    if (msgs.length === 0) continue;

    const sessionId = file.replace('.jsonl', '');
    const dateMatch = file.match(/(\d{4}-\d{2}-\d{2})/);
    const date = dateMatch ? dateMatch[1] : new Date().toISOString().split('T')[0];

    const ok = await postToXavier('/memory/add', {
      content: `Session: ${sessionId}\nDate: ${date}\n\n${msgs.join('\n---\n')}`,
      path: `openclaw/sessions/${sessionId}`,
      metadata: { type: 'session-log', source: 'openclaw', session_id: sessionId, date, synced_at: new Date().toISOString() }
    });

    if (ok) synced++;
    await delay(500);
  }

  return synced;
}

// ===== 4. SWAL QUESTIONS =====
async function syncQuestions() {
  const file = path.join(SWAL_DASHBOARD, 'questions', 'URGENT.md');
  if (!fs.existsSync(file)) return 0;

  const content = fs.readFileSync(file, 'utf8');
  const blocks = content.split('---').filter(b => b.includes('**ID:**'));
  let synced = 0;

  for (const block of blocks) {
    const id = (block.match(/\*\*ID:\*\* (Q-\d+)/) || [])[1];
    const title = (block.match(/\[.*\] (.*?)\n/) || [])[1] || 'Unknown';
    if (!id) continue;

    const ok = await postToXavier('/memory/add', {
      content: block.trim().substring(0, 2000),
      path: `sweat-operations/questions/${id}`,
      metadata: { type: 'question', source: 'swal-operations-dashboard', question_id: id, title: title.trim(), synced_at: new Date().toISOString() }
    });
    if (ok) synced++;
    await delay(200);
  }
  return synced;
}

// ===== 5. SWAL DECISIONS =====
async function syncDecisions() {
  const dirs = [path.join(SWAL_DASHBOARD, 'decisions', 'PENDING.md'), path.join(SWAL_DASHBOARD, 'decisions', 'RESOLVED.md')];
  let synced = 0;

  for (const file of dirs) {
    if (!fs.existsSync(file)) continue;
    const content = fs.readFileSync(file, 'utf8');
    const blocks = content.split('---').filter(b => b.includes('### D-'));

    for (const block of blocks) {
      const id = (block.match(/### (D-\d+):/) || [])[1];
      const title = (block.match(/### D-\d+: (.*)/) || [])[1] || 'Unknown';
      const status = (block.match(/Status:\*\* (PROPOSED|ACCEPTED|DEPRECATED|RESOLVED)/i) || [])[1] || 'proposed';
      if (!id) continue;

      const ok = await postToXavier('/memory/add', {
        content: block.trim().substring(0, 2000),
        path: `sweat-operations/decisions/${id}`,
        metadata: { type: 'decision', source: 'swal-operations-dashboard', decision_id: id, title: title.trim(), status: status.toLowerCase(), synced_at: new Date().toISOString() }
      });
      if (ok) synced++;
      await delay(200);
    }
  }
  return synced;
}

// ===== MAIN =====
const GLOBAL_TIMEOUT_MS = 110000;

async function main() {
  const timeoutId = setTimeout(() => {
    console.log('\n⏰ TIMEOUT (110s) - exiting');
    process.exit(0);
  }, GLOBAL_TIMEOUT_MS);

  process.on('SIGTERM', () => { clearTimeout(timeoutId); process.exit(0); });
  process.on('SIGINT', () => { clearTimeout(timeoutId); process.exit(0); });

  console.log('🔄 SWAL Xavier - Minimal Robust Sync\n');

  let total = 0;

  console.log('📄 MEMORY.md...');
  const m = await syncMemoryMd();
  console.log(`   ✅ ${m} sections`);
  total += m;

  console.log('\n📅 Daily files...');
  const d = await syncDailyFiles();
  console.log(`   ✅ ${d} files`);
  total += d;

  console.log('\n💬 Sessions...');
  const s = await syncSessions();
  console.log(`   ✅ ${s} sessions`);
  total += s;

  console.log('\n❓ Questions...');
  const q = await syncQuestions();
  console.log(`   ✅ ${q} questions`);
  total += q;

  console.log('\n⚖️  Decisions...');
  const dec = await syncDecisions();
  console.log(`   ✅ ${dec} decisions`);
  total += dec;

  clearTimeout(timeoutId);
  console.log(`\n🎉 Done! Total: ${total} items`);
}

main().catch(e => { console.error('Error:', e.message); process.exit(1); });

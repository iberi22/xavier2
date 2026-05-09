#!/usr/bin/env node
/**
 * SWAL Operations Dashboard - Xavier Sync
 * Sincroniza Questions, Decisions y Projects con Xavier
 */

const XAVIER_URL = process.env.XAVIER_URL || 'http://localhost:8003';
const fs = require('fs');
const path = require('path');

function getRequiredXavierToken() {
  const token = process.env.XAVIER_TOKEN || process.env.XAVIER_API_KEY || process.env.XAVIER_TOKEN;
  if (!token) {
    throw new Error('Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.');
  }
  return token;
}

const XAVIER_TOKEN = getRequiredXavierToken();

const DASHBOARD_BASE = 'E:\\scripts-python\\SWAL-Operations-Dashboard';

async function syncToXavier(type, data) {
  const endpoint = `${XAVIER_URL}/memory/add`;
  try {
    await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Xavier-Token': XAVIER_TOKEN },
      body: JSON.stringify({
        path: `sweat-operations/${type}/${data.id}`,
        content: JSON.stringify(data, null, 2),
        metadata: { type, source: 'swal-operations-dashboard', synced_at: new Date().toISOString() }
      })
    });
  } catch (e) { console.log(`  ⚠️ ${type}/${data.id}: ${e.message}`); }
}

async function loadQuestions() {
  const file = path.join(DASHBOARD_BASE, 'questions', 'URGENT.md');
  if (!fs.existsSync(file)) return [];
  const content = fs.readFileSync(file, 'utf8');
  const questions = [];
  const blocks = content.split('---').filter(b => b.includes('**ID:**'));
  for (const block of blocks) {
    const idMatch = block.match(/\*\*ID:\*\* (Q-\d+)/);
    const titleMatch = block.match(/\[.*\] (.*?)\n/);
    if (idMatch) {
      questions.push({
        id: idMatch[1],
        title: titleMatch ? titleMatch[1].trim() : 'Unknown',
        content: block.substring(0, 500),
        priority: 'urgent',
        category: 'tecnico',
        status: 'open',
        project: 'Synapse',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      });
    }
  }
  return questions;
}

async function loadDecisions() {
  const file = path.join(DASHBOARD_BASE, 'decisions', 'PENDING.md');
  if (!fs.existsSync(file)) return [];
  const content = fs.readFileSync(file, 'utf8');
  const decisions = [];
  const blocks = content.split('---').filter(b => b.includes('### D-'));
  for (const block of blocks) {
    const idMatch = block.match(/### (D-\d+):/);
    const titleMatch = block.match(/### D-\d+: (.*)/);
    const statusMatch = block.match(/Status:\*\* (PROPOSED|ACCEPTED|DEPRECATED)/i);
    if (idMatch) {
      decisions.push({
        id: idMatch[1],
        title: titleMatch ? titleMatch[1].trim() : 'Unknown',
        context: block.substring(0, 500),
        decision: '', consequences_positive: [], consequences_negative: [],
        status: statusMatch ? statusMatch[1].toLowerCase() : 'proposed',
        priority: 'high',
        created_at: new Date().toISOString()
      });
    }
  }
  return decisions;
}

async function main() {
  console.log('🔄 SWAL Operations - Xavier Sync\n');

  const questions = await loadQuestions();
  for (const q of questions) { await syncToXavier('questions', q); console.log(`  ✅ ${q.id}`); }
  console.log(`Questions: ${questions.length}\n`);

  const decisions = await loadDecisions();
  for (const d of decisions) { await syncToXavier('decisions', d); console.log(`  ✅ ${d.id}`); }
  console.log(`Decisions: ${decisions.length}\n`);

  console.log('✅ Sync complete!');
}

main().catch(console.error);

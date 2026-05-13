#!/usr/bin/env node

/**
 * Rate Limit Manager CLI
 *
 * Usage:
 *   node rate-manager.js status
 *   node rate-manager.js track <provider> <tokens> <status>
 *   node rate-manager.js sync <provider>
 */

const http = require('http');

const XAVIER_URL = process.env.XAVIER_URL || 'http://localhost:8003';
const XAVIER_TOKEN = process.env.XAVIER_TOKEN || 'dev-token';

const PROVIDERS = ['opencode-go', 'deepseek', 'groq', 'openrouter', 'google', 'openai', 'anthropic'];

async function xavierRequest(path, method = 'GET', body = null) {
  return new Promise((resolve, reject) => {
    const url = new URL(`${XAVIER_URL}${path}`);
    const options = {
      method,
      headers: {
        'X-Xavier-Token': XAVIER_TOKEN,
        'Content-Type': 'application/json'
      }
    };

    const req = http.request(url, options, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        if (res.statusCode >= 400) {
          reject(new Error(`Xavier API error: ${res.statusCode} ${data}`));
        } else {
          try {
            resolve(JSON.parse(data));
          } catch (e) {
            resolve(data);
          }
        }
      });
    });

    req.on('error', reject);
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

async function status() {
  console.log('\n--- Rate Limit Manager Dashboard ---');
  console.log(`Time: ${new Date().toISOString()}\n`);

  for (const provider of PROVIDERS) {
    try {
      const status = await xavierRequest(`/v1/usage/status/${provider}`);
      const weeklyUsage = ((status.used_weekly / status.weekly_quota) * 100).toFixed(1);
      const alert = status.used_weekly > (status.weekly_quota * 0.8) ? '⚠️' : '✅';

      console.log(`${provider.padEnd(15)} | Hourly: ${status.used_hourly.toString().padStart(8)} | Daily: ${status.used_today.toString().padStart(8)} | Weekly: ${weeklyUsage.padStart(5)}% ${alert}`);

      if (status.rate_limited_until) {
        const until = new Date(status.rate_limited_until);
        if (until > new Date()) {
          console.log(`  🛑 RATE LIMITED UNTIL: ${until.toISOString()}`);
        }
      }
    } catch (e) {
      // Provider might not have usage data yet
      console.log(`${provider.padEnd(15)} | No data available`);
    }
  }
  console.log('\nAlert: ⚠️ indicates weekly usage > 80%\n');
}

async function track(provider, tokens, statusCode) {
  try {
    await xavierRequest('/v1/usage/track', 'POST', {
      provider,
      tokens: parseInt(tokens),
      status: parseInt(statusCode)
    });
    console.log(`Tracked ${tokens} tokens for ${provider} (status: ${statusCode})`);
  } catch (e) {
    console.error(`Error tracking usage: ${e.message}`);
  }
}

async function sync(provider) {
  try {
    console.log(`Fetching daily summary for ${provider}...`);
    const summary = await xavierRequest(`/v1/usage/summary/${provider}`);

    const date = new Date().toISOString().split('T')[0];
    const path = `usage/${provider}/${date}`;

    console.log(`Saving summary to memory at ${path}...`);
    await xavierRequest('/memory/add', 'POST', {
      path,
      content: JSON.stringify(summary, null, 2),
      metadata: {
        kind: 'usage_summary',
        provider,
        date,
        daily_total: summary.daily_total,
        daily_tokens: summary.daily_tokens
      }
    });

    console.log('Sync complete.');
  } catch (e) {
    console.error(`Error syncing usage to memory: ${e.message}`);
  }
}

const [,, cmd, ...args] = process.argv;

async function main() {
  switch (cmd) {
    case 'status':
      await status();
      break;
    case 'track':
      if (args.length < 3) {
        console.log('Usage: node rate-manager.js track <provider> <tokens> <status>');
        process.exit(1);
      }
      await track(...args);
      break;
    case 'sync':
      if (args.length < 1) {
        console.log('Usage: node rate-manager.js sync <provider>');
        process.exit(1);
      }
      await sync(...args);
      break;
    default:
      console.log('Usage: node rate-manager.js [status|track|sync]');
      process.exit(1);
  }
}

main().catch(console.error);

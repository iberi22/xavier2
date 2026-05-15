const fs = require('fs');
const path = require('path');

// Load API key from .env
const envPath = path.join(__dirname, '../../.env');
let apiKey = '';

if (fs.existsSync(envPath)) {
    const env = fs.readFileSync(envPath, 'utf-8');
    const match = env.match(/JULES_API_KEY=(.*)/);
    if (match) apiKey = match[1].trim();
}

if (!apiKey) {
    console.error('JULES_API_KEY not found in .env');
    process.exit(1);
}

const API_BASE = 'https://jules.googleapis.com/v1alpha';

async function listSessions() {
    try {
        const response = await fetch(`${API_BASE}/sessions`, {
            headers: {
                'X-Goog-Api-Key': apiKey,
                'Content-Type': 'application/json'
            }
        });

        if (!response.ok) {
            throw new Error(`API Error: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        console.log('--- Jules API Sessions ---');
        console.log(JSON.stringify(data, null, 2));
    } catch (error) {
        console.error('Request failed:', error.message);
    }
}

listSessions();

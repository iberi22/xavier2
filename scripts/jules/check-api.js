const fs = require('fs');
const path = require('path');

const envPath = path.join(__dirname, '../../.env');
let apiKey = '';

if (fs.existsSync(envPath)) {
    const env = fs.readFileSync(envPath, 'utf-8');
    const match = env.match(/JULES_API_KEY=(.*)/);
    if (match) apiKey = match[1].trim();
}

if (!apiKey) {
    console.error('❌ JULES_API_KEY not found in .env');
    process.exit(1);
}

async function validate() {
    try {
        const response = await fetch('https://jules.googleapis.com/v1alpha/sessions', {
            headers: { 'X-Goog-Api-Key': apiKey }
        });

        if (response.ok) {
            console.log('✅ JULES_API_KEY is valid and connected to Google Cloud.');
        } else {
            console.error(`❌ API Error: ${response.status} ${response.statusText}`);
        }
    } catch (error) {
        console.error('❌ Validation failed:', error.message);
    }
}

validate();

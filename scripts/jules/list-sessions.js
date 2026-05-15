const { execSync } = require('child_process');

try {
    console.log('Fetching Jules remote sessions...');
    const output = execSync('jules remote list --session', { encoding: 'utf-8' });
    console.log('--- Current Jules Sessions ---');
    console.log(output);
} catch (error) {
    console.error('Error fetching sessions:', error.message);
}

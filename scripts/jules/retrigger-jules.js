const { execSync } = require('child_process');

const issueNumber = process.argv[2];

if (!issueNumber) {
    console.error('Usage: node retrigger-jules.js <issue_number>');
    process.exit(1);
}

const REPO = 'iberi22/xavier';

try {
    console.log(`Re-triggering Jules for issue #${issueNumber} in ${REPO}...`);

    // Remove label
    console.log('Removing jules label...');
    execSync(`gh issue edit ${issueNumber} --repo ${REPO} --remove-label jules`);

    // Wait a bit
    console.log('Waiting for sync...');
    execSync('powershell Start-Sleep -s 2');

    // Add label
    console.log('Re-adding jules label...');
    execSync(`gh issue edit ${issueNumber} --repo ${REPO} --add-label jules`);

    console.log('Success! Jules should pick up a fresh session now.');
} catch (error) {
    console.error('Operation failed:', error.message);
}

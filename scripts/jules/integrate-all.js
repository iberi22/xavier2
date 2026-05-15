const { execSync } = require('child_process');

const REPO = 'iberi22/xavier';
const TARGET_BRANCH = 'integration';

function run(cmd) {
    console.log(`> ${cmd}`);
    return execSync(cmd, { stdio: 'inherit', shell: 'powershell' });
}

try {
    console.log(`--- Starting Xavier Autonomous Integrator ---`);

    // 1. Get Jules PRs
    console.log('Fetching open PRs...');
    const prsJson = execSync(`gh pr list --repo ${REPO} --json number,title,body,isDraft`, { encoding: 'utf-8' });
    let allPrs = JSON.parse(prsJson);

    // Filter PRs created by Jules
    let prs = allPrs.filter(pr => !pr.isDraft && pr.body.includes('Jules'));

    if (prs.length === 0) {
        console.log('No ready Jules PRs found. Integration complete.');
        process.exit(0);
    }

    console.log(`Found ${prs.length} PRs to integrate.`);

    // 2. Prepare Integration branch
    console.log(`Preparing branch: ${TARGET_BRANCH}`);
    try {
        run(`git checkout -B ${TARGET_BRANCH} origin/main`);
    } catch (e) {
        run(`git checkout -B ${TARGET_BRANCH} main`);
    }

    // 3. Sequential Merge
    let successCount = 0;
    for (const pr of prs) {
        console.log(`\nMerging PR #${pr.number}: ${pr.title}`);
        try {
            // Attempt merge
            run(`gh pr merge ${pr.number} --merge --repo ${REPO} --delete-branch=false`);
            
            // Validate build
            console.log('Validating build...');
            run('cargo check');
            
            successCount++;
            console.log(`✅ Successfully integrated PR #${pr.number}`);
        } catch (error) {
            console.error(`❌ Failed to integrate PR #${pr.number}. Rolling back...`);
            run('git reset --hard HEAD~1');
        }
    }

    console.log(`\n--- Integration Summary ---`);
    console.log(`Total Integrated: ${successCount}/${prs.length}`);
    
    if (successCount === prs.length) {
        console.log('All PRs integrated successfully. Ready for final tests and push to main.');
    } else {
        console.warn('Some PRs failed to merge or broke the build. Manual intervention or AI resolution required.');
    }

} catch (error) {
    console.error('Critical failure in integrator:', error.message);
}

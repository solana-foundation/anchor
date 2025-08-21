# Security Policy

1. [Reporting security problems](#reporting)
2. [Incident Response Process](#process)

<a name="reporting"></a>
## Reporting security problems in the <project_name>

**DO NOT CREATE A GITHUB ISSUE** to report a security problem.

Instead please email disclosures@solana.org.
Provide a helpful title, detailed description of the vulnerability and an exploit
proof-of-concept. Speculative submissions without proof-of-concept will be closed
with no further consideration.


If you haven't done so already, please **enable two-factor auth** in your GitHub account.

Expect a response as fast as possible in the advisory, typically within 72 hours.

--

If you do not receive a response in the advisory, send an email to
disclosures@solana.org with the full URL of the advisory you have created.  DO NOT
include attachments or provide detail sufficient for exploitation regarding the
security issue in this email. **Only provide such details in the advisory**.


If you do not receive a response from disclosures@solana.org please followup with
the team on another platform like @solana_devs on X/twitter

<a name="process"></a>
## Incident Response Process

In case an incident is discovered or reported, the following process will be
followed to contain, respond and remediate:

### 1. Accept the new report
In response a newly reported security problem, a member of the
`solana-foundation/admins` group will accept the report to turn it into a draft
advisory.  The `solana-foundation/security-incident-response` group should be added to
the draft security advisory, and create a private fork of the repository (grey
button towards the bottom of the page) if necessary.

If the advisory is the result of an audit finding, follow the same process as above but add the auditor's github user(s) and begin the title with "[Audit]".

If the report is out of scope, a member of the `solana-foundation/admins` group will
comment as such and then close the report.

### 2. Triage
Within the draft security advisory, discuss and determine the severity of the issue. If necessary, members of the solana-foundation/security-incident-response group may add other github users to the advisory to assist.
If it is determined that this is not a critical network issue then the advisory should be closed and if more follow-up is required a normal Solana public github issue should be created.

### 3. Prepare Fixes
For the affected branches, typically all three (edge, beta and stable), prepare a fix for the issue and push them to the corresponding branch in the private repository associated with the draft security advisory.
There is no CI available in the private repository so you must build from source and manually verify fixes.
Code review from the reporter is ideal, as well as from multiple members of the core development team.

### 4. Notify Security Group Validators
Once an ETA is available for the fix, a member of the solana-foundation/security-incident-response group should notify the validators so they can prepare for an update using the "Solana Red Alert" notification system.
The teams are all over the world and it's critical to provide actionable information at the right time. Don't be the person that wakes everybody up at 2am when a fix won't be available for hours.

### 5. Ship the patch
Once the fix is accepted it may be distributed directly to validators as a patch, depending on the vulnerability.

We currently do not use the Github workflow to publish security advisories. Once the issue and fix have been disclosed, and a bounty category is assessed if appropriate, the GitHub security advisory is no longer needed and can be closed.


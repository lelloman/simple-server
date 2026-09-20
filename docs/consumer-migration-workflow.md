# Reusable consumer migration workflow

Use this workflow for every shared capability and every target service. It was
agreed for the Step 03a rollout and also applies to subsequent migration steps.

## 1. Establish applicability and branch ownership

Read the capability contract and repository instructions. Inspect actual entry
points, configuration, dependencies, custom layers, and tests. Record which
production binaries use the behavior and how they configure it.

Check both sides of applicability:

- Before migration, does the service use or need the capability? For logging,
  inspect subscriber/logger installation and emitted events; do not infer this
  solely from a transitive dependency or test helper.
- After migration, does production startup actually call the shared feature's
  API? Adding a dependency or enabling a Cargo feature is insufficient.

If the capability is unused, record N/A with evidence instead of adding behavior
just to fill a table. If a used capability has an unsupported requirement, record
the gap and coordinate a shared API extension or leave it pending/partial. Do
not remove existing behavior or call an incompatibility N/A.

Inspect local branches, current branch, tracking configuration, recent commits,
and existing worktrees. Choose the active development branch (`master` or `dev`;
use `main` where established). Remote HEAD alone is not sufficient. Resolve
genuine ambiguity before choosing a branch. Record its starting commit and the
original worktree's status, including untracked files.

## 2. Create an isolated migration worktree

Create a uniquely named temporary branch and worktree from the chosen branch.
Perform all implementation, test additions, documentation, and commits there.
Do not edit the original worktree's application files during implementation.

Preserve sibling path dependencies in the worktree layout. For these consumers,
a sibling worktree under `/home/lelloman/lelloprojects` usually retains the
expected path to `simple-server`. Record the exact reviewed library revision.
Do not silently repoint a consumer at another agent's in-progress library code.

Run relevant baseline checks before edits. Finish them before changing tested
sources; distinguish documented baseline failures from migration regressions.
Use isolated test databases, ports, and fixtures. Never use production or user
data for destructive tests. Keep parallel builds within available memory/disk
and avoid sharing a build directory concurrently between different checkouts.

## 3. Implement and verify actual adoption

Preserve configuration semantics, defaults, output contracts, and application
ownership. Migrate the relevant production entry points, not just tests.
Document intentionally unmigrated binaries and custom behavior.

Use meaningful before/after or contract tests for behavior at risk. For 03a,
compare filtering, output destination/format, ANSI, span context, and log bridges
in fresh processes. Use the application's actual configuration parser in those
comparisons: for example, `EnvFilter::new("")` and
`EnvFilter::try_from_default_env()` with an empty variable do not have the same
semantics. Normalize an empty accepted directive set to explicit `off` when the
shared initializer requires a nonempty filter. Verify that child probes really
ran and that startup smoke tests reach the production initializer; `--help` can
exit before initialization. Exercise real startup/shutdown where relevant. Run the
repository's applicable tests, formatting, lint and build checks; record
pre-existing failures and checks not rerun without claiming a fully green suite.

Inspect the final diff and dependency graph. Commit only migration changes and
their documentation on the temporary branch. Update active CI checkout pins and
build revision files to the reviewed shared-library source; preserve historical
revision references in earlier migration records. Report the migration commit,
baseline and final results, scope, and any remaining limits.

## 4. Integrate into the development branch

Before integration, recheck the original branch and worktree for concurrent
commits and new edits. Preserve any changes that appeared while the migration
was in progress. Retain an explicit recovery ref for the original branch tip
when integration rewrites commits or requires conflict resolution.

Rebase the development branch **onto the migration branch**. This usually only
advances the branch when no intervening commits exist. If new development
commits exist, replay them and resolve conflicts without dropping either the
migration or newer application behavior.

If uncommitted work prevents rebase, preserve it in an identified stash including
untracked files, then restore and verify it afterwards. Do not include unrelated
work in migration commits. Avoid stashing files being actively edited by another
agent without coordinating first. Prefer a clean linked worktree for branch
integration when practical, but never force-update a branch checked out elsewhere.

Verify branch ancestry and integration results. An unchanged tested tree needs
only ancestry/tree verification; new conflict resolutions or intervening changes
require relevant checks. Restore the original user's working state. If restoration
or integration is unresolved, retain the migration worktree/branch and report the
issue instead of declaring completion or deleting recovery material.

Only after successful integration and verification, remove the temporary
worktree with `git worktree remove` and delete its fully integrated branch with
`git branch -d`. Do not force-delete unmerged branches or uncommitted work.
Do not remove pre-existing worktrees, branches, or stashes belonging to others.

Local history integration does not authorize publishing rewritten remote
history. Pushes, deployment, and branch-protection changes require separate
authorization.

## 5. Update the shared adoption trackers

Always update both:

- `docs/migration-status.html` — the service/module matrix and explanatory notes.
- `docs/migration-status.md` — the matching status and detailed evidence.

Use Done only for verified production adoption in the development branch.
Use Partial for an explicitly incomplete component scope, Pending for unresolved
work, and N/A only for a capability not used/needed, with the reason recorded.
Do not mark other modules in an umbrella step done automatically.

Record service, module, applicability evidence, development branch, reviewed
library revision, consumer commit, tests and limitations, and cleanup outcome.
Validate tracker links and HTML script syntax, then commit the tracker updates.

When delegating, give each agent this workflow, its specific repositories, the
capability contract, and the reviewed library revision. A single coordinator
owns shared-library changes and central tracker edits. Agents must send their
results to that coordinator so the mandatory updates are not lost or conflicted.

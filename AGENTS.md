# Consumer migration rules

For migrations of `simple-server` capabilities into services, follow
[`docs/consumer-migration-workflow.md`](docs/consumer-migration-workflow.md).
These rules apply to delegated work as well as work performed directly.

- Verify that the service uses the capability and that the shared implementation
  preserves the required behavior. A Cargo feature alone is not adoption.
- Inspect the active development branch (`master` or `dev`; `main` where that is
  the repository's established development branch). Do not assume remote HEAD
  identifies the active branch.
- Implement in a new branch in an isolated worktree based on that branch.
- Test and commit there, rebase the original development branch onto the
  migration branch, verify integration, then remove the temporary worktree and
  branch. Preserve concurrent commits and all unrelated uncommitted work.
- Always update both `docs/migration-status.html` and
  `docs/migration-status.md` with truthful status and verification evidence.
- During multi-agent work, assign disjoint repositories and one owner for the
  central trackers. Agents report evidence to that owner; they do not race to
  edit shared status files or the shared library.
- Do not push, deploy, or alter branch protection unless separately authorized.

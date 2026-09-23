# Consumer reassessment after dynamic cron support

Read-only review of the 16 consumers other than Fausto, against simple-server
`0cff4b2` (`CronRegistry`). No consumer code, configuration or dependencies changed;
no consumer tests were rerun. Findings describe checked-out source, not deployment.
Existing unrelated work in LelloAuth, SCT and Simple AI was left untouched.

## Result

The new component separates cron timing from execution and therefore gives
applications a more flexible integration option. It does not make every
background-work loop a cron use case. Fausto remains the immediate candidate.
None of the other services should switch to CronRegistry solely because it exists.

| Consumer | Reviewed branch / commit | Effect on integration plan |
| --- | --- | --- |
| Pezzottflix | master / `2d49435` | Correct earlier applicability assessment: its cron engine starts but has no production job registrations. The live SQLite job queue remains 06b Pending. CronRegistry is a possible future option, not a current production migration. |
| Observo | master / `9ce6b07` | Keep shared CronSchedule calculations and SQLite-owned timing. A registry would duplicate state refreshed from the database every minute. |
| Pezzottify | dev / `5ea9ed6b` | Keep adopted interval/capacity/policy primitives. Cron and combined-cron branches are still unimplemented; the registry could help a separately requested cron feature. |
| Crumbles | master / `a74f4dd` | No change: SQLite reservations, project/priority ordering and paused-session capacity ownership are the scheduling problem. |
| Favzetto | master / `b8f5260` | No change: persisted assistant claims, due times, priority and per-user/workflow rules remain authoritative. |
| Meteonesto | master / `080a852` | No change: weighted task selection, lease renewal/fencing and hot configuration are not cron timing. |
| SCT | master / `6a999be` | No change: PostgreSQL due times, SKIP LOCKED claims and server/worker epochs stay authoritative. |
| Simple Agents | main / `401b649` | No change: persisted fleet reservations, live runner generations, profile constraints and workspace budgets are the relevant capability. |
| Simple AI | master / `d3103f8` | No change: model routing, batch readiness and runner capacity are request-driven. |
| Pezzottify Downloader | master / `e10c767` | No change: foreground/normal/prefetch ordering, queue bounds and cancellation pruning are capacity admission. |
| LelloAuth | master / `8122dc8` | Keep shared webhook capacity/retry primitives; the delivery queue does not use cron. |
| Quentin Torrentino | master / `4229de2` | Keep separate shared conversion/placement capacities; processing is driven by pipeline work. |
| LelloStore | master / `1a6611e` | No cron use: periodic metrics refresh remains a lifecycle-owned interval. |
| Androidoscopy | master / `49bfea4` | No cron use: discovery, heartbeat and reconnect timers belong to transport/protocol loops. |
| Paranza | master / `1c559ee` | No cron use: the directly awaited PCM maintenance loop and structured runner sessions retain lifecycle ownership. |
| Peerlo | master / `e3b08f6` | No cron use: protocol operations retain their existing ownership; shared retry calculations remain appropriate. |

## Pezzottflix: correct the earlier gap description

[Production startup](../../pezzottflix/pezzottflix-server/src/main.rs) creates and
starts the cron scheduler. Repository-wide Rust call-site inspection finds
`add_job` calls only inside the scheduler's test module. Production API handlers
submit work to [JobQueue](../../pezzottflix/pezzottflix-server/src/jobs/queue.rs),
including content enrichment. Eight workers select SQLite jobs by pending status,
`run_at` eligibility and priority/creation order, and conditionally claim jobs.
Cron API presence and a running empty loop are not production cron adoption.

Consequently 06b stays **Pending (durable queue)**, rather than Pending for an
actively used dynamic cron requirement. The old report remains historical;
this review supersedes that applicability explanation. Migrating an unused cron
engine would not complete the actual queue integration.

If cron becomes a product requirement, the existing
[engine](../../pezzottflix/pezzottflix-server/src/jobs/scheduler.rs) supplies useful
compatibility cases for shared-library design:

- A new/replaced job starts with no last-run time and checks a 365-day historical
  lookback. This can trigger immediately; the registry's default anchor is future-only.
- When no semaphore permit is available, last-run time does not advance. An
  established overdue job remains eligible on the next poll. CronRegistry consumes
  a tick on delivery, before downstream admission.
- Execution limits, including zero, and available-permit reporting can stay
  outside CronRegistry. They no longer require expanding the bounded Scheduler
  just to reuse cron timing.
- Existing cron parser grammar must still be preserved. Registry controls do not
  expand CronSchedule's six/seven-field input grammar.

This suggests a future **peek/commit or admission-aware cursor** API if an active
consumer needs it: inspect an occurrence, advance only after accepted admission,
reject stale commits after replacement/removal, and choose an explicit initial
anchor. This is a proposal, not implemented behavior or durable/exactly-once
execution. Passing a historical anchor already allows historical selection with
the current pure API, but there is no acknowledgement operation. Avoid building
additional shared machinery solely for this currently unused cron path.

## Observo: keep the stateless recurrence primitive

Its [scheduler](../../observo/observo-server/src/scheduler.rs) reloads enabled task
schedules and latest run creation times from SQLite every minute. First-run
selection begins at epoch; invalid historical timestamps use a year lookback.
It creates a pending run before emitting the event. Parser aliases are preserved
by canonicalizing the existing parser's fields into shared CronSchedule.

The database's latest run time changes after manual/external runs too. A separate
registry would need to synchronize those changes, schedule edits and failures to
insert runs. Existing stateless `next_after` calls already provide shared timing
without a second authority. No integration change is recommended.

## Pezzottify: potential feature, not missed migration

The production [scheduler](../../pezzottify/pezzottify-server/src/background_jobs/scheduler.rs)
returns no next occurrence for the cron-only branch and logs that cron is not
implemented; combined schedules still calculate only their interval portion.
Introducing CronRegistry here would activate new behavior, requiring product
choices about startup/restart catch-up, cron/interval composition and persisted
schedule state. Existing shared interval, capacity and execution-policy adoption
remains valid and unchanged.

## Remaining production evidence

The non-cron conclusions were checked against the existing migration reports and
current source, including:

- Crumbles `crumbles-integration/src/scheduler.rs`: SQLite reservation selection
  and the profile/global distinction for paused sessions.
- Favzetto `backend/src/assistant/`: claimable-ticket queries and the worker loop.
- Meteonesto `weather-pipeline/src/task_runtime.rs`: claim/renew/fence and hot
  configuration paths.
- SCT `crates/sct-core/src/maintenance.rs`: database clocks and fenced claims.
- Simple Agents `crates/simple-agents-service/src/fleet.rs`: transactional fleet
  admission and resident workspace budgets.
- Simple AI `backend/src/gateway/scheduler.rs`: model planning, batching and routing.
- Downloader `src/downloader/scheduler.rs`: priority/prefetch queues and permits.
- LelloAuth `crates/lello-auth-core/src/webhook.rs`: shared ExecutionCapacity.
- Quentin `crates/core/src/processor/pipeline.rs`: independent stage capacities.
- LelloStore `backend/src/metrics.rs`, Androidoscopy `server/src/{control,discovery}.rs`,
  Paranza `apps/paranza-server/src/main.rs`, and Peerlo's Rust modules: periodic or
  protocol operations, with no production cron scheduler found.

The library should continue exposing independently usable timing, admission,
capacity and policy components. Durable services may adopt those pieces while
keeping database claims and recovery authoritative. CronRegistry does not resolve
those other gaps, and this review does not authorize changing consumer behavior.

## Verification and status

Source/call-site inspection only; no runtime or consumer test claim. Both central
trackers retain their status counts, with Pezzottflix's Pending reason corrected.
Documentation links, HTML script/rendering, table consistency and whitespace are
checked for this documentation-only update. Work follows an isolated branch from
simple-server/main `8e84e6b`; no push or deployment is part of the review.

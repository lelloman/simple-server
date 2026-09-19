# Step 02: lifecycle and entry-point setup

Status: implementation committed at `c535907`, adopted locally by Favzetto,
LelloStore, LelloAuth, Fausto, Meteonesto, Pezzottify, Observo, Paranza, and Peerlo.
This is the first capability extraction after Step 01 (Axum centralization).

## Outcome

Products keep their `main()` and Tokio runtime. They can replace duplicated
signal handling and server shutdown orchestration with small shared helpers.
An optional coordinator keeps all participating services running until shutdown,
requests that they drain together, and bounds the wait with one explicit deadline.

The first design consumers are Favzetto and Pezzottify. Favzetto validates the
single-listener API and cooperative worker integration. Library tests validate
multiple listeners and peer-address extraction; Pezzottify now coordinates both
listeners, scheduler jobs, maintenance, and tracked application tasks. LelloStore was subsequently authorized as another pilot and
validates two listeners, an application worker, and tracked WebSocket draining.

## Evidence from the first consumers

Historical design evidence, before migration. Reviewed local checkouts: Favzetto `382cc8f`, Pezzottify `e3e69de7`.

| Consumer | Pre-migration behavior | Requirement |
| --- | --- | --- |
| Favzetto `backend/src/main.rs` | Owns config, logging, migrations, database, storage, routes, and an assistant worker; one HTTP listener uses graceful shutdown without a deadline | Preserve the migrate command and startup sequence; replace signal duplication and bound draining |
| Pezzottify `pezzottify-server/src/main.rs` and `src/server/bootstrap.rs` | Owns application startup, a cancellable scheduler, API and metrics listeners; nested `select!` blocks drop sibling serving futures on exit | Coordinate both listeners and keep the scheduler future polled during shutdown; preserve peer-address connection information |

Favzetto's worker is started without retaining a handle in `main()`. Pezzottify
also starts detached maintenance tasks. Neither integration may claim that HTTP
draining alone proves all background work has stopped. Pezzottify's scheduler
currently waits with a timeout for each job; this is not an overall process
shutdown deadline.

## Scope and ownership

Shared building blocks:

- Explicit OS signal registration: SIGINT and SIGTERM on Unix, Ctrl+C elsewhere.
- Cloneable, one-shot shutdown notification usable by HTTP and application code.
- TCP listener binding, including an already-bound listener and port zero.
- HTTP serving with graceful shutdown, retaining transitional Axum compatibility.
- Optional coordination of named, long-running service futures.
- One caller-supplied shutdown budget and structured completion/error results.

Applications retain CLI/config parsing, logging installation, runtime creation,
startup order, database setup and migrations, routers and middleware, background
job implementations, cleanup order, and process exit policy. Library functions
never call `process::exit`, install logging, or create a Tokio runtime.

Task spawning, restart policies, job supervision, readiness endpoints, telemetry,
database abstractions, TLS, Unix sockets, and socket activation are later work.
A long-running scheduler future can participate in coordination without moving
its job management into this library.

## Public surface

Add an opt-in `lifecycle` Cargo feature, leaving existing defaults unchanged.
Core coordination requires no Axum dependency; TCP/HTTP helpers are available
when both `lifecycle` and `http` are enabled. Tokio dependencies are optional and
enabled only for the selected capabilities.

The following illustrates composition with application-provided values. See
`examples/lifecycle.rs` for a complete compiling example and the public modules
for exact generic bounds. Errors retain original sources and service names.

```rust,ignore
// Register explicitly during application startup; registration can fail.
let signals = Signals::install()?;

// No implicit timeout default. The application selects its budget.
let mut lifecycle = Lifecycle::new(ShutdownOptions {
    grace_period: Duration::from_secs(30),
});
let shutdown = lifecycle.shutdown();

// Binding happens before services run. SocketAddr and host:port are supported.
let listener = http::bind(bind_address).await?;
let address = listener.local_addr()?;
let server = http::serve(listener, app, shutdown.clone());
lifecycle.service("http", server)?;

// Optional: register an application-owned scheduler future as another service.
// It observes shutdown, performs its own drain, and returns a Result.
// lifecycle.service("scheduler", run_scheduler(shutdown.clone()))?;

let report = lifecycle.run(signals.wait(), async {
    // Polled only after every registered service has finished draining.
    // Application-owned cleanup, e.g. closing a database pool.
    cleanup().await
}).await?;
```

`Shutdown` exposes `request()`, `is_requested()`, and `requested().await`.
Requests are idempotent and sticky: late subscribers observe shutdown immediately.
Dropping a notification handle does not request shutdown. Notification is
runtime-neutral to callers, without requiring them to adopt a public
`tokio_util::CancellationToken` type. Existing tokens can be cancelled by a small
application-owned bridge that is polled alongside its scheduler.

`Signals::wait()` returns `ShutdownReason::Signal` or an I/O error. It is optional:
`Lifecycle::run` also accepts an application-provided trigger future, allowing
tests, admin actions, and embedded callers to avoid process signal handlers.
Programmatic `Shutdown::request()` also wakes the coordinator independently of
the supplied trigger. Custom triggers return `Result<ShutdownReason, E>`; use
`ShutdownReason::Triggered` for normal application triggers. Signal registration
is process-wide and explicitly opt-in;
dropping the helper must not be documented as restoring default OS behavior.

`http::serve` must support both an ordinary Axum router and
`into_make_service_with_connect_info::<SocketAddr>()`, and accept a pre-bound
Tokio TCP listener. The serving future observes `Shutdown`; it does not install
signals or choose a deadline. Applications may use this helper independently of
the coordinator, or retain their existing Axum serving call.

The coordinator polls registered futures concurrently without silently spawning
them. Registration rejects duplicate names; running with no services is an
error. Futures need not be `'static` merely to participate. Exact type bounds
and error conversion are implemented in `src/lifecycle.rs`. Service futures must
be `Send`; trigger and cleanup futures need not be. Panics propagate to the
caller. Dropping `run` cancels coordination without executing final cleanup.

## Shutdown contract

1. The application prepares dependencies and binds all listeners before running
   the coordinator. Startup failure is returned immediately; the application
   owns cleanup of partially initialized resources. No service accepts requests
   through the coordinator until startup has succeeded.
2. While running, a signal, programmatic request, trigger failure, or completion
   of any registered service begins shutdown. Services are expected to be
   long-running; even an early `Ok(())` is an unexpected-service-exit error.
3. At the first shutdown request, record the cause and one monotonic deadline,
   then notify every participant. Stop accepting new HTTP connections and ask
   existing connections to drain through the HTTP implementation. Continue
   polling all unfinished service futures, including after another one fails.
4. Once all services finish, poll the application's final cleanup future with
   the remaining budget. Dependency-specific ordering inside cleanup belongs to
   the application. Cleanup also runs after a service error if draining finishes
   before the deadline.
5. At the deadline, return a timeout error containing the unfinished service
   names or the cleanup phase. Drop remaining owned futures. Do not start final
   cleanup while services remain unfinished. A zero budget requests shutdown
   immediately when triggered and permits no draining wait.
6. Return success only if shutdown was requested normally and draining and
   cleanup succeeded. Preserve the initiating failure and subsequent failures
   in the returned report/error; do not let a timeout hide a service failure.
   A race between independent causes need not have a deterministic winner, but
   an observed failure must never become a successful shutdown.

The budget covers draining plus final cleanup, not startup or normal runtime.
For a programmatic request, timing starts when the coordinator observes it.
Repeated requests do not extend the deadline. A second signal has no special
force-exit meaning in this step; the application/operator owns forced termination.

This is a cooperative deadline on how long the coordinator waits. Dropping a
serving future or a task handle is not proof that spawned connection tasks,
upgraded WebSockets, detached workers, or blocking work have stopped. A stalled
runtime can also prevent timely deadline polling. Applications own cancellation
and joining of that work and decide how to exit after a timeout. Never describe
the timeout as forcibly closing every socket or terminating every task.

Streaming responses and SSE can hold HTTP drain open until the deadline.
WebSocket handlers and other upgraded connections must explicitly observe the
application's shutdown notification and have their completion tracked by the
application if they are to count toward a fully drained service.

## Integration sequence

1. Implement and test the notification, signals, coordinator, and HTTP adapters
   in `simple-server`, including standalone composition examples.
2. Integrate Favzetto: preserve the migrate command and setup order, adopt shared
   signals and HTTP drain, and select an explicit budget. Audit worker and stream
   ownership; document remaining detached work or add application-owned
   cancellation/joining before claiming full application cleanup.
3. Integrate Pezzottify: bind API and metrics listeners before serving, register
   both servers and the scheduler, bridge scheduler cancellation, and replace
   the nested `select!` shutdown path. Preserve peer-address extraction and audit
   detached maintenance tasks and long-lived connections.
4. Validate both consumers, then migrate other products individually, recording
   their budgets, participating services, cleanup behavior, and remaining limits.

Each budget must fit the product's deployment termination grace period with
room for process teardown. Thirty seconds in the example is illustrative.
Mark library availability separately from adoption in each product. Favzetto's
scope and known baseline failures are recorded in its migration documentation.

## Acceptance checks

- Notification is idempotent, wakes multiple subscribers, and handles requests
  made before waiting. Independent lifecycle instances do not interfere.
- A real HTTP request completes during draining; new connections stop being
  accepted once the listener has entered shutdown. Port-zero binding and
  already-bound listeners work, and bind failures retain their source.
- Two listeners and a cooperative scheduler drain together. Early service exit
  and service failure both initiate shutdown without dropping sibling futures
  before they have had a chance to drain.
- One deadline covers all services and cleanup. A stuck request or service
  returns a timeout identifying unfinished work; cleanup ordering, cleanup
  failure, trigger failure, zero budgets, and repeated requests are covered.
- SIGINT and Unix SIGTERM are tested in child processes. Core tests use injected
  triggers, so they do not install signal handlers in the test runner.
- Connection-info support compiles and preserves the peer address. Real streaming
  and WebSocket cases demonstrate the documented drain/cancellation limits.
- Check default, lifecycle-only, HTTP+lifecycle, all-features, and
  no-default-features builds. Run the existing library checks and each consumer's
  required checks, including real-server shutdown tests.

Both original pilot integrations are implemented. Each consumer documents its
shutdown scope and validation; adoption across all products is tracked separately.

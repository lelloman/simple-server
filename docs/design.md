# Design direction

This document records the agreed end goal and design direction, not a finalized
API specification.

## End goal: completely abstract Axum away

Consumer services must eventually use only `simple-server`'s public HTTP
interfaces. Axum may remain the internal implementation, but consumers must not
need to import, name, implement, or understand Axum APIs to build their services.
This includes routing, handlers, state, extractors, responses, errors, middleware,
multipart uploads, streaming, and WebSockets wherever those capabilities are used.

The abstraction is complete when:

- Consumer production code and tests no longer use `simple_server::axum`, direct
  Axum dependencies, or Axum-specific companion APIs.
- Public interfaces expose no Axum types, traits, trait bounds, or errors,
  including through aliases or re-exports.
- Existing service behavior is preserved and verified through the shared APIs.
- The transitional Axum re-export and any temporary Axum escape hatches are
  removed. Missing capabilities are addressed by extending the shared API.

Dependency centralization and completion of the infrastructure modules are
intermediate milestones. A final consumer/API audit must verify these criteria
before the overall abstraction is marked complete. The API design and migration
sequence remain incremental; this goal does not require replacing Axum internally.

The [Step 02 lifecycle contract](step-02-lifecycle.md) defines the first capability
extraction, its implemented API, shutdown contract, and pilot acceptance checks.

[Step 03: observability](step-03-observability.md) follows with three independent
modules: logging setup (03a), request correlation (03b), and HTTP tracing (03c).
The [03a contract](step-03a-logging.md) describes the implemented logging module and
Favzetto pilot. The [03b contract](step-03b-correlation.md) defines request IDs
and the Crumbles compatibility pilot. The [03c contract](step-03c-http-tracing.md)
defines request spans and response-body lifecycle events.

[Step 04](step-04-http-policy-assessment.md) separates body limits (04a), response
headers (04b) and CORS configuration (04c). The implemented
[04a contract](step-04a-body-limits.md) preserves extractor-limit semantics.
The [04b contract](step-04b-response-headers.md) defines framework-independent
header defaults, replacement and lossless Vary merging.
The [04c contract](step-04c-cors.md) provides optional CORS configuration with
framework-independent public types and explicit policy.

[Step 05: health and readiness](step-05-health.md) provides optional ordered
application checks and a framework-independent endpoint adapter. Applications
retain dependency policy and response contracts.

## Next milestones (updated 2026-09-23)

Step 07 (optional database helpers) is deferred until after Step 10 and the
consumer Axum-removal milestone. Keep the existing step numbers so historical
references remain valid. The execution order after Step 06 is:

1. Steps 08/09: one optional `auth` module for authentication and authorization,
   designed and migrated together. Step 09 is absorbed; there is no second rollout.
   Completed locally: 16 adopted and one N/A; see the
   [auth rollout evidence](migration-status.md#steps-0809-final-auth-rollout--2026-09-23).
2. Step 10: rate limiting (next).
3. Complete the public HTTP interfaces, migrate all remaining consumer Axum
   usage, and verify the end-goal criteria above before removing the transitional
   Axum re-export. This includes production code and tests across all services.
4. Revisit Step 07: optional database setup and migration helpers.

Database helpers are not a prerequisite for the HTTP abstraction. Services retain
their existing database libraries, setup and migrations while that work proceeds.
Axum may remain internal to simple-server; the milestone removes its exposure to
consumers. Completing Steps 08–10 alone does not establish that milestone.

## Composition

Axum remains the internal HTTP implementation. The long-term product API hides
Axum types, traits, extractors, and errors. When a service requires an additional
capability, extend the public API and adapt it to Axum internally.

Migration is incremental. First centralize the dependency and temporarily expose
`simple_server::axum`, preserving the existing routing and execution flow.
Extract shared capabilities one at a time across services, then retire exposed
Axum APIs as the library's interfaces cover their needs.

Keep modules independently usable. A service should not need a database,
authentication provider, or background worker to use server lifecycle helpers.
No module requires handing over control of `main()`. An optional lifecycle
coordinator will compose the same public building blocks available to products.

## Responsibilities

| Shared foundation | Application |
| --- | --- |
| Listener setup and shutdown coordination | Startup dependencies and cleanup |
| Logging setup, request IDs, HTTP tracing | Domain events and audit semantics |
| Health and readiness endpoint plumbing | Readiness conditions and dependency checks |
| Optional metrics and middleware helpers | Routes, state, and business behavior |
| Rate-limit policies, key extraction, and middleware | Limit scope, trusted proxies, keys, and storage choice |
| Optional database setup and migration helpers | Database library, schema, queries, and transactions |
| Task ownership, scheduling and execution policies | Background jobs, durable claims and recovery |
| Optional authentication and authorization composition | Identity providers, permission models, and policy |

Shutdown deadlines, long-lived requests, and worker cleanup need explicit
semantics. Readiness must reflect application conditions; process liveness alone
does not establish readiness. Middleware should remain configurable for routes
such as uploads, streaming responses, and WebSockets.

## Adoption

1. Centralize Axum on the exact 0.8.9 pin, migrating services individually from
   their existing 0.7/0.8 dependencies. Check companion crates, feature needs,
   route semantics, WebSockets, and existing tests for each service.
2. Use Pezzottify and Favzetto as the first design consumers. Pezzottify has
   custom rusqlite stores and more complex authentication/workers; Favzetto has
   a SQLx SQLite pool and API-key authentication.
3. Extract lifecycle and entry-point setup, retaining product ownership of
   `main()`. Apply each shared capability incrementally across services.
4. Adopt observability through 03a, 03b, and 03c independently, then follow with
   further HTTP support, health, background tasks, authentication, authorization,
   and rate limiting as requirements are validated. Database helpers are deferred
   until after the consumer Axum-removal milestone.
5. Replace all transitional Axum usage with product-facing interfaces, verify
   the end-goal criteria above across consumers, and remove the Axum re-export.

Version the library so each product can upgrade independently. Repository
creation does not imply a release, a finalized licensing decision, or a
workspace-wide migration.

[Step 06: background tasks](step-06-background-tasks.md) separates task ownership,
scheduling and execution policies. Durable storage and recovery remain application-owned.

[Steps 08/09: auth](step-08-auth.md) combine identity verification and access checks
in one optional, Axum-independent module with application-owned providers and policy.

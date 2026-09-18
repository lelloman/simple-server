# Design direction

This document records the initial direction, not a finalized API specification.

## Composition

Build a library around Axum's existing programming model. Accept ordinary Axum
routers rather than introducing a separate handler, routing, or application
framework. Applications initialize their dependencies and provide their router
and explicit server configuration.

Keep modules independently usable. A service should not need a database,
authentication provider, or background worker to use server lifecycle helpers.

## Responsibilities

| Shared foundation | Application |
| --- | --- |
| Listener setup and shutdown coordination | Startup dependencies and cleanup |
| Logging setup, request IDs, HTTP tracing | Domain events and audit semantics |
| Health and readiness endpoint plumbing | Readiness conditions and dependency checks |
| Optional metrics and middleware helpers | Routes, state, and business behavior |
| Future task cancellation and supervision | Background job implementations |
| Future authentication adapters | Authorization and permission rules |

Shutdown deadlines, long-lived requests, and worker cleanup need explicit
semantics. Readiness must reflect application conditions; process liveness alone
does not establish readiness. Middleware should remain configurable for routes
such as uploads, streaming responses, and WebSockets.

## Adoption

1. Select two existing services with different operational needs.
2. Choose the supported Axum version and document the migration implications.
   Existing services currently span Axum 0.7 and 0.8.
3. Extract lifecycle, logging, tracing, and health helpers through those
   integrations. Test observable behavior, including shutdown and readiness.
4. Stabilize a small API before migrating additional services.
5. Add further modules only when real integrations justify the abstraction.

Version the library so each product can upgrade independently. Repository
creation does not imply a release, a finalized licensing decision, or a
workspace-wide migration.

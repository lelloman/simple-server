# Design direction

This document records the initial direction, not a finalized API specification.

The [Step 02 lifecycle contract](step-02-lifecycle.md) defines the first capability
extraction, its implemented API, shutdown contract, and pilot acceptance checks.

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
| Future task cancellation and supervision | Background job implementations |
| Future authentication and authorization building blocks | Identity providers, permission models, and policy |

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
4. Follow with HTTP support, observability, health, background tasks, databases,
   authentication, authorization, and rate limiting as requirements are validated.
5. Replace transitional Axum usage with product-facing interfaces over time.

Version the library so each product can upgrade independently. Repository
creation does not imply a release, a finalized licensing decision, or a
workspace-wide migration.

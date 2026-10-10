# Lunar (lunarphp/lunar) — Feature Dossier
Registry: #126 | Category: 08 | Verified: 3,683 stars, MIT, last push 2026-08-24

## Feature inventory
- **Headless commerce as Laravel packages, not a standalone app**: "a set of Laravel packages that bring functionality akin to Shopify and other e-commerce platforms to Laravel"; complete storefront freedom with the backend handled; integrates directly into any Laravel project — no separate system to maintain, no external API to call, no vendor lock-in [VERIFIED-fetched:https://raw.githubusercontent.com/lunarphp/lunar/main/README.md]
- **Monorepo of two package families**: Admin hub (Laravel Livewire; manage products, orders, staff members) + Core (all models, actions and utilities, required by the hub) [VERIFIED-fetched:https://raw.githubusercontent.com/lunarphp/lunar/main/README.md]
- **Composer-level split**: `lunarphp/core` is the e-commerce engine (models, pipelines, pricing, search, core functionality); `lunarphp/lunar` is the admin panel built on Filament; the Livewire-based admin imposes no requirement on the storefront [VERIFIED-fetched:https://docs.lunarphp.io/]
- **Key feature set**: product catalog (products, variants, options, collections, associations, flexible attribute system); cart & checkout with built-in pipelines for validation, tax calculation, and shipping; order management (full lifecycle, statuses, transactions, PDF support); discounts (percentage, fixed amount, buy-X-get-Y with conditions and limitations); payments (first-party Stripe, PayPal, Opayo drivers plus an extensible payment interface); search (Algolia and Meilisearch through Laravel Scout with pre-configured indexing); multi-channel and multi-currency with localized pricing; taxation (tax classes, zones, rates); extensible Filament admin (pages, resources, relation managers) [VERIFIED-fetched:https://docs.lunarphp.io/]
- **Requirements**: PHP ^8.1, Laravel 9+, MySQL 8.0+ / PostgreSQL 9.2+ [VERIFIED-fetched:https://raw.githubusercontent.com/lunarphp/lunar/main/README.md]
- **1.x transition churn**: the 0.8 branch has reached end of life — no PRs or support accepted due to a new admin panel framework and significant architecture changes; migration to latest 1.x recommended [VERIFIED-fetched:https://raw.githubusercontent.com/lunarphp/lunar/main/README.md]
- **Livewire starter kit** ships as a fully working storefront starting point alongside storefront guides for catalog, cart, and checkout flows [VERIFIED-fetched:https://docs.lunarphp.io/]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Commerce function suite (catalog / cart / orders / payments / discounts / tax) | 🏛 | Commerce dept candidate (new) | A whole business function per the rubric — if LastHarness ever transacts physical or licensed goods, this maps the dept's minimum record scope: products, orders, transactions, taxes each as gated memory domains. |
| Built-in checkout pipelines (validation → tax → shipping as composable stages) | 🧩 | §12 / §16 kernel transaction staging | Ordered stages that can each veto the transaction before commit = decision-log-friendly order flow; the pipeline pattern ports to any kernel-mediated exchange needing staged approval. |
| Headless split (engine vs storefront; admin imposes nothing downstream) | 🧩 | §4.4 kernel mediation | Consumers integrate through the package's API and never reach past it into internals — same mediation discipline as kernel-routed work where UIs never touch worker state directly. |
| Payment driver abstraction with first-party Stripe/PayPal/Opayo drivers | ⚙ | Finance dept via §7.1 gateway | Processor calls are external execution — adoptable only as individually allow-listed gateway connectors behind the driver interface, which keeps swap-ins bounded and logged. |
| Filament-based extensible admin hub | ⚙ | §16.10 Owner Surfaces | Generated-admin-over-records console with customizable pages/resources is an owner-review UX precedent for LastHarness's owner console. |
| Meilisearch driver (self-hostable) | ⚙ | Ops; in-boundary | Scout-backed catalog search can run entirely inside the Linux boundary. |
| Algolia driver | ❌ | — | SaaS-hosted indexing executes off-boundary; excluded under local-first unless replaced by the Meilisearch path. |
| MySQL 8.0+/PostgreSQL requirement | ⚙ | Ops isolation note | Second database engine tolerable ONLY as dept-tool-local state in its own container; never on kernel paths, which stay SQLite-direct. |

## Notable absences
No storefront included by design (headless) — starter kit notwithstanding, everything customer-facing is yours to build. No managed-hosting story beyond composer install; queue/worker topology not described in primary sources. No agent/AI integration anywhere. The 0.8→1.x break with an EOL branch and admin-framework swap signals real upgrade-churn risk for anything built against it. Payment processors imply unavoidable third-party financial egress regardless of self-hosting.

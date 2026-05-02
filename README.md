# Emakao — Property Management Platform

Emakao is a multi-tenant property management SaaS backend built for the Kenyan market. It is written entirely in Rust using the Axum web framework and is designed to serve property agencies of all sizes — from independent landlords on the Starter plan to large multi-branch firms on the Enterprise plan.

---

## Table of Contents

- [What it does](#what-it-does)
- [Tech stack](#tech-stack)
- [Architecture](#architecture)
- [Project structure](#project-structure)
- [Database design](#database-design)
- [Subscription plans & feature flags](#subscription-plans--feature-flags)
- [Authentication & authorisation](#authentication--authorisation)
- [Local development setup](#local-development-setup)
- [Environment variables](#environment-variables)
- [Make commands](#make-commands)
- [Database workflow](#database-workflow)
- [Building for production](#building-for-production)
- [CI without a live database](#ci-without-a-live-database)
- [Adding a migration](#adding-a-migration)
- [Adding a new tenant (agency)](#adding-a-new-tenant-agency)

---

## What it does

Emakao provides the complete backend for a property management agency:

| Domain | Capabilities |
|---|---|
| **Properties & Units** | Create and manage residential, commercial, community, student, and affordable housing portfolios. Units support sub-units, floor plans, and vacancy tracking. |
| **Residents** | Tenant profiles, portal invitations, national ID, and contact management. |
| **Owners** | Property owner profiles, KRA PIN, bank accounts, M-Pesa numbers, ownership percentages per property. |
| **Agreements / Leases** | Tenancy agreements linking residents to units; tracks start/end dates, rent amounts, deposits, billing frequency, and lifecycle status (Draft → Active → Terminated). |
| **Payments** | M-Pesa STK Push, paybill, and till integrations via Safaricom Daraja. Manual payment claims with proof-of-payment upload (S3). Review and approval workflow. |
| **Ledger** | Double-entry ledger per agency: charges (rent, deposit, HOA dues, utility, late fees), payments (M-Pesa, bank, cash), credits, and waivers. Outstanding balance computed from immutable entries. |
| **Invoices** | Line-item invoices with tax, linked to agreements and residents. |
| **Maintenance** | Work orders with priority (Low / Medium / High / Emergency) and vendor assignment. Status lifecycle: Open → In Progress → Completed / Cancelled. |
| **Vendors** | Agency-scoped contractor directory. Linked to work orders. |
| **Utility Billing** | Utility meters (electricity, water, gas) with pre-paid and post-paid billing modes. Reading history and generated bills. |
| **Messaging** | Conversations with participant lists and messages, backed by Socket.IO for real-time delivery. |
| **Applicants** | Rental application pipeline with status tracking and income verification fields. |
| **Disbursements** | Owner payment runs via bank transfer, M-Pesa B2C, or cheque. |
| **Accounting** | Chart of accounts, journal entries, VAT reports, management fee calculations, bank reconciliation. |
| **Subscriptions** | Three-tier SaaS subscription (Starter / Growth / Enterprise) with feature flags, per-plan limits, per-agency overrides, trial periods, grace periods, and M-Pesa STK push billing. |
| **Communications** | SMS via Africa's Talking, WhatsApp, email via SMTP (Lettre). |
| **Documents** | File storage on S3-compatible object store (MinIO in dev). |

---

## Tech stack

| Layer | Technology |
|---|---|
| Web framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| Async runtime | [Tokio](https://tokio.rs) |
| Database ORM | [sqlx](https://github.com/launchbadge/sqlx) 0.8 — compile-time query macros with offline cache |
| Primary database | PostgreSQL 16 |
| Caching / queues | [Redis](https://redis.io) 7 via [fred](https://github.com/aembke/fred.rs) + [Apalis](https://github.com/geofmureithi/apalis) |
| Real-time messaging | [socketioxide](https://github.com/Totodore/socketioxide) |
| Auth | JWT ([jsonwebtoken](https://github.com/Keats/jsonwebtoken)) + Argon2 password hashing |
| Fine-grained authorisation | [OpenFGA](https://openfga.dev) (gRPC via Tonic) |
| Payments | Safaricom Daraja API (M-Pesa STK Push, Paybill, B2C) |
| SMS | [Africa's Talking](https://africastalking.com) |
| Email | SMTP via [Lettre](https://lettre.rs) |
| File storage | AWS S3 / [MinIO](https://min.io) |
| Validation | [garde](https://github.com/jprochazk/garde) |
| API docs | [utoipa](https://github.com/juhaku/utoipa) + Swagger UI |
| Serialisation | serde / serde_json |
| Logging/tracing | tracing + tracing-subscriber |
| Containerisation | Docker + Docker Compose |

---

## Architecture

The backend follows **hexagonal (ports and adapters) architecture** with four layers:

```
src/
├── domain/          Pure business logic — no I/O, no frameworks.
│                    Entities, value objects, domain errors, commands.
│
├── application/     Use cases and port interfaces (traits).
│                    Orchestrates domain logic. Knows nothing about HTTP or DB.
│
├── infrastructure/  Adapters: sqlx repos, Redis cache, OpenFGA, M-Pesa client,
│                    S3 storage, SMTP, Africa's Talking SMS.
│
└── presentation/    Axum handlers, middleware, routing, request/response DTOs.
                     Translates HTTP ↔ application layer.
```

Dependencies flow inward only: `presentation → application → domain`. Infrastructure implements the ports defined in `application/ports/`.

---

## Project structure

```
emakao/
├── Cargo.toml                    Workspace root + main crate
├── Makefile                      All developer commands
├── Dockerfile                    Multi-stage production image
├── docker-compose.yml            Local dev services
├── docker/
│   └── init-db.sql               Creates emakao_tenant database on first run
│
├── migrations/
│   ├── platform/                 Platform DB migrations (agencies, users, plans…)
│   └── tenant/                   Tenant DB migrations (properties, residents…)
│
├── crates/
│   └── migrate/                  Standalone migration binary (no compile-time macros)
│       ├── Cargo.toml
│       └── src/main.rs
│
├── src/
│   ├── main.rs                   Entry point — wires config, state, router
│   ├── config.rs                 All env-var config in one struct
│   ├── lib.rs
│   │
│   ├── domain/                   Pure domain models and commands
│   │   ├── agency.rs
│   │   ├── agreement.rs
│   │   ├── applicant.rs
│   │   ├── auth.rs
│   │   ├── conversation.rs
│   │   ├── disbursement.rs
│   │   ├── errors.rs
│   │   ├── inspection.rs
│   │   ├── invoice.rs
│   │   ├── ledger.rs
│   │   ├── maintenance.rs
│   │   ├── message.rs
│   │   ├── owner.rs
│   │   ├── payment.rs
│   │   ├── property.rs
│   │   ├── resident.rs
│   │   ├── subscription.rs
│   │   ├── unit.rs
│   │   ├── utility.rs
│   │   └── vendor.rs
│   │
│   ├── application/
│   │   ├── errors.rs             AppError enum
│   │   ├── ports/                Repository traits (interfaces)
│   │   └── use_cases/            Business use-case implementations
│   │
│   ├── infrastructure/
│   │   ├── auth/                 JWT issue + verify
│   │   ├── cache/                Redis pool + helpers
│   │   ├── db/                   sqlx repository implementations
│   │   │   ├── pool.rs           Platform + tenant pool management
│   │   │   ├── agency_repository_sqlx.rs
│   │   │   ├── agreement_repository_sqlx.rs
│   │   │   ├── auth_repository_sqlx.rs
│   │   │   ├── conversation_repository_sqlx.rs
│   │   │   ├── ledger_repository_sqlx.rs
│   │   │   ├── maintenance_repository_sqlx.rs
│   │   │   ├── message_repository_sqlx.rs
│   │   │   ├── owner_repository_sqlx.rs
│   │   │   ├── payment_repository_sqlx.rs
│   │   │   ├── property_repository_sqlx.rs
│   │   │   ├── resident_repository_sqlx.rs
│   │   │   ├── subscription_repository_sqlx.rs
│   │   │   ├── utility_repository_sqlx.rs
│   │   │   └── vendor_repository_sqlx.rs
│   │   ├── email/                Lettre SMTP client
│   │   ├── openfga/              gRPC client for fine-grained authz
│   │   ├── payments/             Safaricom Daraja M-Pesa client
│   │   ├── sms/                  Africa's Talking client
│   │   └── storage/              AWS S3 / MinIO client
│   │
│   └── presentation/
│       ├── app_state.rs          AppState — all dependencies wired here
│       ├── router.rs             Axum router — all routes defined here
│       ├── middleware/
│       │   ├── auth.rs           JWT extraction + AuthenticatedUser extractor
│       │   └── tenant_resolver.rs  Resolves agency from subdomain/header
│       └── handlers/             One module per domain
│
└── .sqlx/                        Offline query cache (committed to git)
```

---

## Database design

Emakao uses **two separate PostgreSQL databases** to cleanly separate platform concerns from tenant data:

### `emakao_platform` — shared infrastructure

The `public` schema holds everything that is global to the SaaS platform:

| Table | Purpose |
|---|---|
| `agencies` | One row per customer agency. Stores `slug`, `schema_name`, `fga_store_id`. |
| `users` | All users across all agencies. Scoped by `agency_id`. |
| `subscription_plans` | Starter, Growth, Enterprise plan definitions. |
| `subscriptions` | Active subscription per agency with status, period, trial dates. |
| `plan_features` | Per-plan feature flags (boolean/integer/string values). |
| `plan_limits` | Per-plan numeric limits (`max_properties`, `max_units`, etc.) |
| `feature_overrides` | Admin-controlled per-agency feature overrides (with expiry). |
| `subscription_invoices` | Payment history for subscription billing. |
| `pending_mpesa_subscription_requests` | STK push intent tracking. |

### `emakao_tenant` — per-agency data

Each agency gets its **own Postgres schema** (e.g. `acme_realty`) inside the tenant database. Every tenant schema gets the full set of tables from `migrations/tenant/`:

| Table | Purpose |
|---|---|
| `properties` | Property portfolio |
| `units` | Units within properties |
| `owners` | Property owners |
| `property_owners` | Many-to-many: owner ↔ property with ownership percentage |
| `residents` | Tenant / resident profiles |
| `agreements` | Tenancy agreements |
| `payment_claims` | Manual payment submissions |
| `ledger_entries` | Immutable financial ledger |
| `work_orders` | Maintenance work orders |
| `vendors` | Contractor directory |
| `utility_meters` | Utility meters per unit |
| `meter_readings` | Reading history |
| `utility_bills` | Generated utility bills |
| `conversations` | Messaging conversations |
| `messages` | Individual messages |
| `applicants` | Rental application pipeline |
| `invoices` | Tenant-facing invoices |
| `disbursements` | Owner payment runs |

### How multi-tenancy works at runtime

1. Every incoming request includes an agency identifier (subdomain or header).
2. The `tenant_resolver` middleware looks up the agency's `schema_name` from the platform DB and injects a `ResolvedAgency` into the request extensions.
3. The `TenantPoolManager` (backed by a `DashMap`) returns a per-agency `PgPool` with `search_path = <schema>, public` — so all sqlx queries run against the right schema transparently.
4. Platform queries (subscriptions, auth) use the platform pool directly.

---

## Subscription plans & feature flags

Three tiers ship out of the box:

| | Starter | Growth | Enterprise |
|---|---|---|---|
| Users | 5 | 25 | Unlimited |
| Properties | 10 | 60 | Unlimited |
| Units | 50 | 300 | Unlimited |
| M-Pesa STK | ✓ | ✓ | ✓ |
| WhatsApp | ✗ | ✓ | ✓ |
| Owner portal | ✗ | ✓ | ✓ |
| Digital signing | ✗ | ✓ | ✓ |
| Double-entry accounting | ✗ | ✓ | ✓ |
| White label | ✗ | ✗ | ✓ |
| API access | ✗ | ✓ | ✓ |

Features and limits are stored in the database (`plan_features`, `plan_limits`), not hardcoded in source. Per-agency overrides can be applied by admins with optional expiry dates, enabling custom deals without code changes.

The `AgencyEntitlements` struct is the canonical in-memory representation of what an agency is allowed to do. Handlers check it via `entitlements.has_feature(&FeatureKey::XYZ)` and `entitlements.within_limit(&LimitKey::MaxProperties, current_count)`.

---

## Authentication & authorisation

- **Authentication**: Argon2 password hashing on login. On success a signed JWT is returned containing `sub` (user ID), `agency_id`, `role`, and `jti` (token ID for revocation).
- **Middleware**: The `auth` middleware extracts and validates the JWT on every protected route and injects `AuthenticatedUser` into request extensions.
- **Fine-grained authorisation**: [OpenFGA](https://openfga.dev) is used for relationship-based access control (e.g. "can user X view property Y?"). Each agency gets its own FGA store ID (`agencies.fga_store_id`).
- **Roles**: `admin`, `staff`, `owner`, `resident` — checked in handlers via the role field on `AuthenticatedUser`.

---

## Local development setup

### Prerequisites

- Rust (stable, 1.78+) — [rustup.rs](https://rustup.rs)
- Docker Desktop
- `make`
- `psql` (for `make db-reset` and `make db-create-agency`)

```bash
# Install sqlx CLI (needed for cargo sqlx prepare)
cargo install sqlx-cli --no-default-features --features postgres
```

### First time

```bash
# 1. Clone and enter the project
git clone https://github.com/your-org/emakao.git
cd emakao

# 2. Copy env template and fill in secrets
cp .env.example .env

# 3. Start all services, run migrations, build
make setup
```

`make setup` does the following in order: starts Docker services, waits for Postgres to be healthy, runs all platform and tenant migrations, prints the local service URLs.

Then start the dev server with hot-reload:

```bash
# requires cargo-watch: cargo install cargo-watch
make dev
```

The API will be available at `http://localhost:3000`.

---

## Environment variables

Copy `.env.example` to `.env` and fill in the values. Required variables are marked *.

```bash
# Server
PORT=3000
RUST_LOG=emakao=debug,tower_http=debug,sqlx=warn

# Databases *
PLATFORM_DATABASE_URL=postgres://emakao:password@localhost:5432/emakao_platform
TENANT_DATABASE_URL=postgres://emakao:password@localhost:5432/emakao_tenant
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=2

# Redis *
REDIS_URL=redis://localhost:6379

# JWT *
JWT_SECRET=change-me-to-a-long-random-string
JWT_EXPIRY_HOURS=24

# OpenFGA
OPENFGA_URL=http://localhost:8080

# M-Pesa (Safaricom Daraja)
MPESA_CONSUMER_KEY=
MPESA_CONSUMER_SECRET=
MPESA_SHORTCODE=
MPESA_PASSKEY=
MPESA_CALLBACK_URL=https://your-domain.co.ke/api/v1/payments/mpesa/callback
MPESA_BASE_URL=https://sandbox.safaricom.co.ke   # use api.safaricom.co.ke in prod

# Africa's Talking (SMS)
AT_API_KEY=
AT_USERNAME=sandbox
AT_SENDER_ID=EMAKAO

# AWS / MinIO (S3-compatible)
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
AWS_ENDPOINT_URL=http://localhost:9000   # omit in prod for real AWS
AWS_REGION=af-south-1
S3_BUCKET=emakao

# SMTP
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_FROM=noreply@emakao.co.ke
# SMTP_USERNAME=
# SMTP_PASSWORD=
```

---

## Make commands

```bash
make setup           # First-time: docker up + migrate + build
make dev             # Start dev server with hot-reload (cargo-watch)
make migrate         # Run all platform + tenant migrations
make migrate-tenant  # Re-run tenant migrations only (all schemas)
make migrate-schema  # Run tenant migrations for a single schema (prompts for name)
make build           # cargo build (uses .sqlx offline cache — no DB needed)
make check           # cargo check
make fmt             # cargo fmt
make clippy          # cargo clippy -D warnings
make test            # cargo test
make docker-up       # Start: postgres, redis, minio, mailhog, openfga
make docker-down     # Stop all containers
make db-reset        # ⚠ DESTROYS ALL DATA — drops and re-migrates both DBs
make db-create-agency # Create a new agency row + provision its tenant schema
make sqlx-prepare    # Regenerate .sqlx offline cache (needs live DB)
```

---

## Database workflow

### sqlx compile-time macros and the offline cache

sqlx's `query!` and `query_as!` macros verify SQL against a live database **at compile time**. This gives you type-safe queries but creates a bootstrapping problem: you need a fully migrated DB before you can compile, but you need to compile to run migrations.

The solution is a **two-stage setup**:

1. `crates/migrate` is a **separate workspace crate** with zero compile-time macros. It uses only runtime sqlx calls, so it always compiles without a database.
2. After migrations run, `cargo sqlx prepare` generates a `.sqlx/` directory — a JSON cache of every query's expected schema. This is committed to git.
3. The main crate compiles against `.sqlx/` using `SQLX_OFFLINE=true`. **No database is needed to build** once the cache exists.

### The single-URL trick for `cargo sqlx prepare`

The main crate's `query!` macros don't know about two databases. `cargo sqlx prepare` accepts only one `DATABASE_URL`. The fix: point it at the platform DB but set `search_path = dev_agency, public` so Postgres can resolve all tenant-schema tables too through a single connection:

```bash
DATABASE_URL="postgres://emakao:password@localhost:5432/emakao_platform?options=-c%20search_path%3Ddev_agency,public" \
  cargo sqlx prepare --workspace
```

This is handled automatically by `make sqlx-prepare`.

### Summary of when to run what

| Situation | Command |
|---|---|
| Fresh clone / new machine | `make setup` |
| Added or changed a migration file | `make migrate && make sqlx-prepare` |
| Changed a query in the main crate | `make sqlx-prepare` |
| Schema unchanged, just coding | `make build` (no DB needed) |
| Wiped Docker volumes | `make db-reset` |
| CI build | Just `make build` — `.sqlx/` is committed |

---

## Building for production

```bash
# Build the release binary
cargo build --release

# Or via Docker
docker build -t emakao:latest .
```

The Dockerfile uses a two-stage build: a `rust:1.78-slim-bookworm` builder stage, then a minimal `debian:bookworm-slim` runtime image. The final image exposes port `3000` and contains both the `emakao` server binary and the `migrate` binary.

On container startup, run migrations before starting the server:

```bash
# In your orchestrator / entrypoint script:
./migrate
./emakao
```

The `emakao` service block in `docker-compose.yml` is commented out during development (you run `cargo run` locally), but can be uncommented for staging.

---

## CI without a live database

Because `.sqlx/` is committed, a standard build and test run requires **no database at all**:

```yaml
# .github/workflows/ci.yml (minimal example)
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release   # SQLX_OFFLINE=true via .cargo/config.toml
      - run: cargo test
```

Only regenerate the cache when migrations change. A separate CI job can do this conditionally and commit the updated `.sqlx/` back:

```yaml
  prepare:
    if: contains(github.event.head_commit.modified, 'migrations/')
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: emakao
          POSTGRES_PASSWORD: password
          POSTGRES_DB: emakao_platform
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build -p migrate
      - run: ./target/debug/migrate
        env:
          PLATFORM_DATABASE_URL: postgres://emakao:password@localhost/emakao_platform
          TENANT_DATABASE_URL: postgres://emakao:password@localhost/emakao_tenant
      - run: |
          DATABASE_URL="postgres://emakao:password@localhost/emakao_platform?options=-c%20search_path%3Ddev_agency,public" \
          cargo sqlx prepare --workspace
      - run: |
          git config user.email "ci@emakao.co.ke"
          git config user.name "CI"
          git add .sqlx && git diff --cached --quiet || git commit -m "chore: update sqlx cache" && git push
```

---

## Adding a migration

1. Create a new numbered SQL file in the appropriate directory:
   ```bash
   # Platform migration (agencies, users, subscriptions…)
   touch migrations/platform/0010_add_something.sql

   # Tenant migration (properties, residents, agreements…)
   touch migrations/tenant/0010_add_something.sql
   ```
   sqlx runs migrations in filename-sorted order. Use leading zeros to maintain ordering.

2. Write your SQL. Use `gen_random_uuid()` not `uuid_generate_v4()` (no extension needed). For custom enum columns in queries, use `$N::text::your_enum_type` casts.

3. Apply and regenerate the cache:
   ```bash
   make migrate
   make sqlx-prepare
   ```

4. Commit both the migration file and the updated `.sqlx/` directory.

---

## Adding a new tenant (agency)

In development:

```bash
make db-create-agency
# Prompts: Agency name, Slug
# Creates the agencies row on the platform DB and runs tenant migrations for the new schema
```

In code (at registration time), the `AgencyRepository` handles the INSERT, and the migration binary is called to provision the schema:

```rust
// application layer — simplified
agency_repo.create(&cmd).await?;
// then call the migrate binary / run TENANT_MIGRATOR for the new schema
```

---

## Local service URLs

After `make setup`:

| Service | URL |
|---|---|
| Emakao API | `http://localhost:3000` |
| Swagger UI | `http://localhost:3000/docs` |
| Postgres (platform) | `postgres://emakao:password@localhost:5432/emakao_platform` |
| Postgres (tenant) | `postgres://emakao:password@localhost:5432/emakao_tenant` |
| Redis | `localhost:6379` |
| MinIO console | `http://localhost:9001` — `minioadmin` / `minioadmin` |
| MailHog (SMTP UI) | `http://localhost:8025` |
| OpenFGA HTTP | `http://localhost:8080` |
| OpenFGA gRPC | `localhost:8081` |
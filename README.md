# Financial Ledger API

A production-oriented REST API for tracking personal and organizational financial records, built with Rust, Axum, and SeaORM. The service supports multi-user authentication, role-based access control, soft-deleted audit trails, and dashboard analytics -- all backed by a PostgreSQL or SQLite database.

## Table of Contents

- [Features](#features)
- [Tech Stack](#tech-stack)
- [Prerequisites](#prerequisites)
- [Setup & Configuration](#setup--configuration)
- [Environment Variables](#environment-variables)
- [Running Migrations](#running-migrations)
- [Starting the Server](#starting-the-server)
- [Running Tests](#running-tests)
- [Role Definitions](#role-definitions)
- [API Endpoints](#api-endpoints)
  - [Authentication](#authentication)
  - [Financial Records](#financial-records)
  - [Dashboard](#dashboard)
  - [User Management (Admin Only)](#user-management-admin-only)
- [Request & Response Examples](#request--response-examples)
- [Error Response Format](#error-response-format)
- [Assumptions & Design Choices](#assumptions--design-choices)

---

## Features

- JWT-based authentication with Argon2 password hashing
- Three-tier role system (Viewer, Analyst, Admin)
- Full CRUD operations on financial records with soft deletes
- IDOR protection enforced at the database query level
- Dashboard analytics: income/expense summaries, category breakdowns, trends, and recent activity
- Filterable record listing by type, category, and date range
- Cross-database support: SQLite for testing, PostgreSQL for production
- Comprehensive integration test suite

## Tech Stack

| Layer        | Technology |
|--------------|------------|
| HTTP Framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| ORM          | [SeaORM](https://github.com/SeaQL/sea-orm) 1.1 |
| Async Runtime | [Tokio](https://github.com/tokio-rs/tokio) 1.50 |
| Auth         | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) 10.3 (HS256) |
| Password Hashing | [Argon2](https://github.com/RustCrypto/passwords) 0.5.3 |
| Validation   | [validator](https://github.com/Keats/validator) 0.20 |
| Database     | PostgreSQL (production) / SQLite (testing) |
| Migration    | SeaORM Migration CLI |

## Prerequisites

- **Rust** 1.75+ (Edition 2024)
- **PostgreSQL** 14+ (production) or SQLite (development/testing)
- **cargo** and **cargo run** for building and running migrations

## Setup & Configuration

### 1. Clone the repository

```sh
git clone <repository-url>
cd ledger-service
```

### 2. Configure the environment

Create a `.env` file in the project root. See [Environment Variables](#environment-variables) for the full list.

```sh
cp .env.example .env  # if an example file exists
```

At minimum, set `DATABASE_URL`:

```
DATABASE_URL=postgres://user:password@localhost:5432/ledger_db
```

### 3. Build the project

```sh
cargo build
```

### 4. Run migrations

```sh
cd migration
cargo run
```

### 5. Start the server

```sh
cargo run
```

The server starts on `http://127.0.0.1:3000` by default.

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | -- | Connection string for PostgreSQL or SQLite. Example: `postgres://user:pass@localhost:5432/ledger_db` or `sqlite:ledger.db?mode=rwc` |
| `JWT_SECRET` | No | `super_secret_key` | Secret key for signing JWT tokens (HS256). **Must be set to a strong, unique value in production.** |

## Running Migrations

The migration directory is a standalone Cargo project using `sea-orm-migration`. All migration commands are run from the `migration/` directory:

```sh
cd migration
```

| Command | Description |
|---------|-------------|
| `cargo run` | Apply all pending migrations |
| `cargo run -- up` | Same as above (explicit) |
| `cargo run -- up -n 10` | Apply the first 10 pending migrations |
| `cargo run -- down` | Rollback the last applied migration |
| `cargo run -- down -n 10` | Rollback the last 10 migrations |
| `cargo run -- fresh` | Drop all tables, then reapply all migrations |
| `cargo run -- refresh` | Rollback all, then reapply all |
| `cargo run -- reset` | Rollback all migrations (keep tables) |
| `cargo run -- status` | Show status of all migrations |
| `cargo run -- generate MIGRATION_NAME` | Generate a new migration file |

## Starting the Server

```sh
cargo run
```

The server binds to `127.0.0.1:3000`. On startup it:

1. Loads environment variables via `dotenvy`
2. Connects to the database using `DATABASE_URL`
3. Prints a success or failure message and exits if the connection fails

## Running Tests

```sh
cargo test
```

Tests use an in-memory SQLite database -- no external services are required. The test suite includes:

- Authentication integration (login, token validation, role enforcement)
- Record CRUD with soft-delete verification
- Dashboard summary correctness
- Input validation (negative amounts, empty categories, invalid enums)
- User management by admins

## Role Definitions

The system defines three roles. Each role determines what a user can access and modify. **All data access is scoped to the user's own records** -- there is no cross-user data visibility.

| Capability | Viewer | Analyst | Admin |
|---|---|---|---|
| View own records | Yes | Yes | Yes |
| View dashboard | Yes | Yes | Yes |
| Create records | No | Yes | Yes |
| Update own records | No | Yes | Yes |
| Delete own records (soft) | No | Yes | Yes |
| List all users | No | No | Yes |
| Create users | No | No | Yes |
| Change user roles | No | No | Yes |
| Activate/deactivate users | No | No | Yes |
| Delete (deactivate) users | No | No | Yes |

**New registrations default to the `Viewer` role.** An administrator must explicitly elevate a user to Analyst or Admin.

## API Endpoints

### Authentication

| Method | Path | Auth Required | Description |
|--------|------|---------------|-------------|
| POST | `/auth/register` | No | Register a new user (default role: Viewer) |
| POST | `/auth/login` | No | Authenticate and receive a JWT token |

### Financial Records

| Method | Path | Auth Required | Min Role | Description |
|--------|------|---------------|----------|-------------|
| GET | `/records` | Yes | Viewer | List own records (filterable by type, category, date range) |
| POST | `/records` | Yes | Analyst | Create a new financial record |
| PUT | `/records/{id}` | Yes | Analyst | Update an existing record (partial update supported) |
| DELETE | `/records/{id}` | Yes | Analyst | Soft-delete a record |

### Dashboard

| Method | Path | Auth Required | Description |
|--------|------|---------------|-------------|
| GET | `/dashboard/summary` | Yes | Income and expense totals (excludes Transfer/Adjustment) |
| GET | `/dashboard/categories` | Yes | Totals grouped by category and record type |
| GET | `/dashboard/trends` | Yes | Totals grouped by date and record type |
| GET | `/dashboard/recent` | Yes | The 5 most recent records, ordered by date |

### User Management (Admin Only)

| Method | Path | Auth Required | Min Role | Description |
|--------|------|---------------|----------|-------------|
| GET | `/users` | Yes | Admin | List all users |
| POST | `/users` | Yes | Admin | Create a new user with a specified role and temporary password |
| PUT | `/users/{id}/role` | Yes | Admin | Change a user's role |
| PATCH | `/users/{id}/status` | Yes | Admin | Change a user's status (active, inactive, suspended, deleted) |
| DELETE | `/users/{id}` | Yes | Admin | Deactivate a user (sets status to inactive) |

### Health & Diagnostics

| Method | Path | Auth Required | Description |
|--------|------|---------------|-------------|
| GET | `/health` | No | Returns `200 OK` if the server is running |
| GET | `/db-status` | No | Returns database connectivity status |

## Request & Response Examples

### Register a User

```sh
curl -X POST http://127.0.0.1:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "secure_password_123"
  }'
```

Response (`201 Created`):

```json
{
  "status": "success",
  "message": "User registered successfully",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "alice@example.com",
    "role": "viewer"
  }
}
```

### Login

```sh
curl -X POST http://127.0.0.1:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "secure_password_123"
  }'
```

Response (`200 OK`):

```json
{
  "status": "success",
  "message": "Login successful",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### Create a Financial Record

```sh
curl -X POST http://127.0.0.1:3000/records \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "amount": 1500.00,
    "type": "income",
    "category": "Salary",
    "notes": "Monthly salary",
    "date": "2026-04-01"
  }'
```

Response (`201 Created`):

```json
{
  "status": "success",
  "message": "Record created",
  "record": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount": 1500.00,
    "type": "income",
    "category": "Salary",
    "notes": "Monthly salary",
    "date": "2026-04-01"
  }
}
```

### List Records with Filters

```sh
curl -G "http://127.0.0.1:3000/records" \
  -H "Authorization: Bearer <token>" \
  --data-urlencode "type=income" \
  --data-urlencode "start_date=2026-01-01" \
  --data-urlencode "end_date=2026-04-30"
```

### Get Dashboard Summary

```sh
curl http://127.0.0.1:3000/dashboard/summary \
  -H "Authorization: Bearer <token>"
```

Response (`200 OK`):

```json
{
  "status": "success",
  "summary": {
    "total_income": 15000.00,
    "total_expense": 8200.50
  }
}
```

### Soft-Delete a Record

```sh
curl -X DELETE http://127.0.0.1:3000/records/a1b2c3d4-e5f6-7890-abcd-ef1234567890 \
  -H "Authorization: Bearer <token>"
```

The record is not physically removed. Its `deleted_at` field is set to the current timestamp. Subsequent requests for this record return `404 Not Found`.

### Create a User as Admin

```sh
curl -X POST http://127.0.0.1:3000/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <admin_token>" \
  -d '{
    "email": "bob@example.com",
    "role": "analyst"
  }'
```

Response (`201 Created`):

```json
{
  "status": "success",
  "message": "User created successfully",
  "user": {
    "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
    "email": "bob@example.com",
    "role": "analyst",
    "status": "active",
    "created_at": "2026-04-05T12:00:00Z",
    "temporary_password": "r4nd0mP@ss"
  }
}
```

## Error Response Format

All errors follow a consistent JSON shape:

```json
{
  "error": "Validation failed",
  "details": {
    "detail": "Amount must be greater than 0"
  }
}
```

Field-level validation errors include a `fields` object:

```json
{
  "error": "Validation failed",
  "details": {
    "fields": {
      "amount": ["Amount must be greater than 0"],
      "category": ["Category cannot be empty"]
    }
  }
}
```

| HTTP Status | Error Label | When |
|-------------|-------------|------|
| 401 | Authentication required | Missing, expired, or invalid JWT token |
| 403 | Access denied | Insufficient role for the requested action |
| 404 | Resource not found | Record or user does not exist (or is soft-deleted) |
| 409 | Resource conflict | Duplicate email during registration |
| 422 | Validation failed | Invalid input (bad amount, empty category, malformed JSON, invalid enum) |
| 500 | Internal server error | Database connection failure or unexpected server error |

## Assumptions & Design Choices

This section documents the key architectural decisions made during development, the reasoning behind them, and the trade-offs involved. Understanding these choices will help you navigate the codebase and extend it with confidence.

### Soft Deletes Over Hard Deletes

Financial records are never permanently removed from the database. Instead, the `deleted_at` timestamp column is set to the current time when a user deletes a record. All queries filter on `deleted_at IS NULL` to exclude soft-deleted records from results.

**Why:** In a financial ledger, auditability is critical. Hard deletes destroy evidence of transactions, which is unacceptable in any system that may need to produce audit trails, comply with regulations, or recover from accidental deletions. Soft deletes preserve the full history of every record.

**Trade-off:** Every query must remember to filter on `deleted_at IS NULL`. Forgetting this filter leaks deleted records into results. The database also grows over time, so a periodic archival or purge strategy (not yet implemented) would be needed in production.

Users use a different approach -- a `status` enum column (`active`, `inactive`, `suspended`, `deleted`) -- rather than a `deleted_at` timestamp. Deactivating a user via `DELETE /users/{id}` sets status to `inactive` rather than `deleted`, allowing administrators to temporarily suspend accounts without fully removing them.

### IDOR Protection at the Query Level

Insecure Direct Object Reference (IDOR) attacks are prevented by enforcing ownership checks at the database query layer, not in application logic after fetching records. Every query that operates on a financial record includes both the `record_id` and the `user_id` from the authenticated user's JWT claims:

```rust
.filter(financial_records::Column::Id.eq(record_id))
.filter(financial_records::Column::UserId.eq(user_id))
.filter(financial_records::Column::DeletedAt.is_null())
```

**Why:** This is defense in depth. Even if a handler accidentally exposes a record belonging to another user, the SQL query itself will return zero rows, resulting in a 404 rather than a data leak. The ownership constraint is impossible to bypass because it is baked into every service method signature and query builder chain.

**Trade-off:** The `user_id` parameter must be threaded through every service function, which adds boilerplate. However, this explicitness makes the security model easy to audit -- any query missing the `user_id` filter is immediately visible during code review.

### Exclusion of Transfer and Adjustment Types from Dashboard Totals

The `RecordType` enum defines four variants: `Income`, `Expense`, `Transfer`, and `Adjustment`. When calculating the `DashboardSummary` (total income, total expense, net balance), only `Income` and `Expense` records are included. `Transfer` and `Adjustment` records are explicitly excluded via a `match` statement in application logic after the SQL query returns results.

**Why:** Transfers move money between accounts without changing overall net worth, and adjustments are bookkeeping corrections rather than real financial activity. Including them in income/expense totals would distort financial reports and mislead users reviewing their cash flow.

**Trade-off:** The exclusion happens in Rust code rather than in the SQL query. This means Transfer and Adjustment records are still fetched from the database and grouped by type -- they simply are not accumulated into the summary totals. The `get_category_summary` and `get_trends` endpoints do not apply this exclusion, so Transfer and Adjustment records appear in those views. An alternative approach would be to filter them out at the SQL level with a `WHERE type NOT IN ('transfer', 'adjustment')` clause, but the current approach keeps the query generic and allows the full dataset to be returned for views that need it.

### Role-Based Access Control with Default Viewer Role

The system defines three roles: `Viewer`, `Analyst`, and `Admin`. New users registering through the public registration endpoint are automatically assigned the `Viewer` role, which grants read-only access to their own records. Viewers cannot create, update, or delete records. Analysts can perform full CRUD operations on their own records. Admins have all Analyst permissions plus the ability to manage users (list, create, change roles, activate/deactivate, delete).

**Why:** The principle of least privilege. Most users of a financial system should not be able to modify data -- they should only view it. By defaulting new registrations to `Viewer`, the system prevents unauthorized data modifications until an administrator explicitly grants elevated permissions.

**Trade-off:** Self-registration is somewhat limited in utility since new users start with read-only access. In a real deployment, an admin would need to upgrade users after registration, or the registration flow would need to be redesigned for different user onboarding paths. Role checks are implemented as Axum middleware layers stacked on route groups, which means the auth middleware runs first (outer layer), then the role middleware (inner layer). This ordering is intentional -- there is no point checking a role if the user is not authenticated.

### Centralized Error Handling with `From` Trait Conversions

All errors in the application flow through a single `AppError` enum, with automatic conversions from library-level errors (`sea_orm::DbErr`, `jsonwebtoken::Error`, `validator::ValidationErrors`, Axum's `JsonRejection`) via `From` trait implementations. Each variant maps to an appropriate HTTP status code (401, 403, 404, 409, 422, 500) and produces a consistent JSON response shape.

**Why:** This eliminates scattered `match` statements throughout handlers and ensures every error response looks the same. Adding a new error type is a matter of adding a variant to the enum and implementing `From` for any source error type. Handlers remain focused on business logic rather than error formatting.

**Trade-off:** The `AppError` enum grows as new error scenarios are discovered, and some conversions (like mapping database constraint violations to user-friendly messages) require string matching on error messages, which is fragile. A more robust approach would be to define custom error types at the repository level.

### Custom JSON Extractor for Consistent Deserialization Errors

Instead of using Axum's built-in `Json<T>` extractor directly, the application defines an `AppJson<T>` wrapper that catches JSON parsing and deserialization failures and converts them into `AppError::ValidationErrorWithDetails` responses.

**Why:** By default, Axum's `Json` extractor returns errors as plain text or HTML, which breaks the API's contract of always returning structured JSON. The custom extractor ensures that even malformed request bodies produce parseable error responses.

### SQLite for Testing, PostgreSQL for Production

The migration system and entity definitions support both SQLite and PostgreSQL through SeaORM's abstracted query interface. Integration tests use an in-memory SQLite database, while production deployments target PostgreSQL.

**Why:** SQLite requires no external services, making tests fast, deterministic, and portable. Developers can run the full test suite without Docker or a running database. PostgreSQL is used in production for its concurrency handling, advanced indexing, and reliability under load.

**Trade-off:** Not all PostgreSQL-specific features (custom types, partial indexes, CTEs) work identically in SQLite. If the codebase ever uses database-specific features, the test suite may not catch incompatibilities. Currently, the application avoids such features to maintain cross-database compatibility.

### JWT Secret Fallback for Development

The JWT signing key is read from the `JWT_SECRET` environment variable, but falls back to `"super_secret_key"` if the variable is not set.

**Why:** This allows the application to start without configuration during local development and testing. It eliminates a common friction point for new developers cloning the repository.

**Trade-off:** This is explicitly unsafe for production. Any deployment must set a strong, unique `JWT_SECRET`. A better approach would be to panic at startup if `JWT_SECRET` is missing in production environments, or to use a configuration file that validates required secrets before the server starts.

### Pagination Not Implemented

The `GET /records` endpoint returns all records belonging to the authenticated user without pagination.

**Why:** For the scope of this application, simplicity was prioritized. Pagination adds complexity to both the API contract (page size, cursor, offset) and the frontend consuming the API.

**Trade-off:** Users with thousands of records will experience slow response times and high memory usage. In production, keyset (cursor-based) pagination would be the preferred approach for an append-mostly-immutable ledger, as it avoids the performance pitfalls of offset-based pagination on large tables.

### Single-Endpoint Record Retrieval Not Wired

The `get_record` handler and its corresponding service function (`get_record` in `record_service.rs`) exist in the codebase and are fully implemented, but the handler is not currently wired into the route tree in `main.rs`. This is an oversight from development, not an intentional design choice. The functionality can be enabled by adding a `.route("/{id}", get(handlers::record_handler::get_record))` line to the `read_routes` group.

---

## Project Structure

```
ledger-service/
├── Cargo.toml
├── .env
├── migration/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── m20220101_000001_create_table.rs
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── error.rs
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── users.rs
│   │   ├── financial_records.rs
│   │   ├── record_type.rs
│   │   ├── role.rs
│   │   └── status.rs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── record_handler.rs
│   │   ├── dashboard_handler.rs
│   │   └── user_handler.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── record_service.rs
│   │   ├── dashboard_service.rs
│   │   └── user_service.rs
│   └── middleware/
│       ├── mod.rs
│       └── auth.rs
└── tests/
    ├── auth_integration.rs
    ├── dashboard_integration.rs
    ├── record_api_integration.rs
    ├── record_integration.rs
    ├── user_integration.rs
    └── validation_integration.rs
```

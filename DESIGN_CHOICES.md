## Assumptions & Design Choices

This section documents the key architectural decisions made during development, the reasoning behind them, and the trade-offs involved. Understanding these choices will help you navigate the codebase and extend it with confidence.

### 1. Soft Deletes Over Hard Deletes

Financial records are never permanently removed from the database. Instead, the `deleted_at` timestamp column is set to the current time when a user deletes a record. All queries filter on `deleted_at IS NULL` to exclude soft-deleted records from results.

**Why:** In a financial ledger, auditability is critical. Hard deletes destroy evidence of transactions, which is unacceptable in any system that may need to produce audit trails, comply with regulations, or recover from accidental deletions. Soft deletes preserve the full history of every record.

**Trade-off:** Every query must remember to filter on `deleted_at IS NULL`. Forgetting this filter leaks deleted records into results. The database also grows over time, so a periodic archival or purge strategy (not yet implemented) would be needed in production.

**Note:** Users use a different approach -- a `status` enum column (`active`, `inactive`, `deleted`) -- rather than a `deleted_at` timestamp. Deactivation sets status to `inactive` rather than `deleted`, allowing administrators to temporarily suspend accounts without fully removing them.

### 2. IDOR Protection at the Query Level

Insecure Direct Object Reference (IDOR) attacks are prevented by enforcing ownership checks at the database query layer, not in application logic after fetching records. Every query that operates on a financial record includes both the `record_id` and the `user_id` from the authenticated user's JWT claims:

```rust
.filter(financial_records::Column::Id.eq(record_id))
.filter(financial_records::Column::UserId.eq(user_id))
.filter(financial_records::Column::DeletedAt.is_null())
```

**Why:** This is defense in depth. Even if a handler accidentally exposes a record belonging to another user, the SQL query itself will return zero rows, resulting in a 404 rather than a data leak. The ownership constraint is impossible to bypass because it is baked into every service method signature and query builder chain.

**Trade-off:** The `user_id` parameter must be threaded through every service function, which adds boilerplate. However, this explicitness makes the security model easy to audit -- any query missing the `user_id` filter is immediately visible during code review.

### 3. Exclusion of Transfer and Adjustment Types from Dashboard Totals

The `RecordType` enum defines four variants: `Income`, `Expense`, `Transfer`, and `Adjustment`. When calculating the `DashboardSummary` (total income, total expense, net balance), only `Income` and `Expense` records are included. `Transfer` and `Adjustment` records are explicitly excluded via a `match` statement in application logic after the SQL query returns results.

**Why:** Transfers move money between accounts without changing overall net worth, and adjustments are bookkeeping corrections rather than real financial activity. Including them in income/expense totals would distort financial reports and mislead users reviewing their cash flow.

**Trade-off:** The exclusion happens in Rust code rather than in the SQL query. This means Transfer and Adjustment records are still fetched from the database and grouped by type -- they simply are not accumulated into the summary totals. The `get_category_summary` and `get_trends` endpoints do not apply this exclusion, so Transfer and Adjustment records appear in those views. An alternative approach would be to filter them out at the SQL level with a `WHERE type NOT IN ('transfer', 'adjustment')` clause, but the current approach keeps the query generic and allows the full dataset to be returned for views that need it.

### 4. Role-Based Access Control with Default Viewer Role

The system defines three roles: `Viewer`, `Analyst`, and `Admin`. New users registering through the public registration endpoint are automatically assigned the `Viewer` role, which grants read-only access to their own records. Viewers cannot create, update, or delete records. Analysts can perform full CRUD operations on their own records. Admins have all Analyst permissions plus the ability to manage users (list, create, change roles, activate/deactivate, delete).

**Why:** The principle of least privilege. Most users of a financial system should not be able to modify data -- they should only view it. By defaulting new registrations to `Viewer`, the system prevents unauthorized data modifications until an administrator explicitly grants elevated permissions.

**Trade-off:** Self-registration is somewhat limited in utility since new users start with read-only access. In a real deployment, an admin would need to upgrade users after registration, or the registration flow would need to be redesigned for different user onboarding paths. Role checks are implemented as Axum middleware layers stacked on route groups, which means the auth middleware runs first (outer layer), then the role middleware (inner layer). This ordering is intentional -- there is no point checking a role if the user is not authenticated.

### 5. Centralized Error Handling with `From` Trait Conversions

All errors in the application flow through a single `AppError` enum, with automatic conversions from library-level errors (`sea_orm::DbErr`, `jsonwebtoken::Error`, `validator::ValidationErrors`, Axum's `JsonRejection`) via `From` trait implementations. Each variant maps to an appropriate HTTP status code (401, 403, 404, 409, 422, 500) and produces a consistent JSON response shape.

**Why:** This eliminates scattered `match` statements throughout handlers and ensures every error response looks the same. Adding a new error type is a matter of adding a variant to the enum and implementing `From` for any source error type. Handlers remain focused on business logic rather than error formatting.

**Trade-off:** The `AppError` enum grows as new error scenarios are discovered, and some conversions (like mapping database constraint violations to user-friendly messages) require string matching on error messages, which is fragile. A more robust approach would be to define custom error types at the repository level.

### 6. Custom JSON Extractor for Consistent Deserialization Errors

Instead of using Axum's built-in `Json<T>` extractor directly, the application defines an `AppJson<T>` wrapper that catches JSON parsing and deserialization failures and converts them into `AppError::ValidationErrorWithDetails` responses.

**Why:** By default, Axum's `Json` extractor returns errors as plain text or HTML, which breaks the API's contract of always returning structured JSON. The custom extractor ensures that even malformed request bodies produce parseable error responses.

### 7. SQLite for Testing, PostgreSQL for Production

The migration system and entity definitions support both SQLite and PostgreSQL through SeaORM's abstracted query interface. Integration tests use an in-memory SQLite database, while production deployments target PostgreSQL.

**Why:** SQLite requires no external services, making tests fast, deterministic, and portable. Developers can run the full test suite without Docker or a running database. PostgreSQL is used in production for its concurrency handling, advanced indexing, and reliability under load.

**Trade-off:** Not all PostgreSQL-specific features (custom types, partial indexes, CTEs) work identically in SQLite. If the codebase ever uses database-specific features, the test suite may not catch incompatibilities. Currently, the application avoids such features to maintain cross-database compatibility.

### 8. JWT Secret Fallback for Development

The JWT signing key is read from the `JWT_SECRET` environment variable, but falls back to `"super_secret_key"` if the variable is not set.

**Why:** This allows the application to start without configuration during local development and testing. It eliminates a common friction point for new developers cloning the repository.

**Trade-off:** This is explicitly unsafe for production. Any deployment must set a strong, unique `JWT_SECRET`. A better approach would be to panic at startup if `JWT_SECRET` is missing in production environments, or to use a configuration file that validates required secrets before the server starts.

### 9. Pagination Not Implemented

The `GET /records` endpoint returns all records belonging to the authenticated user without pagination.

**Why:** For the scope of this application, simplicity was prioritized. Pagination adds complexity to both the API contract (page size, cursor, offset) and the frontend consuming the API.

**Trade-off:** Users with thousands of records will experience slow response times and high memory usage. In production, keyset (cursor-based) pagination would be the preferred approach for an append-mostly-immutable ledger, as it avoids the performance pitfalls of offset-based pagination on large tables.

### 10. Single-Endpoint Record Retrieval Not Wired

The `get_record` handler exists in the codebase and the service layer implements fetching a single record by ID, but this handler is not currently wired into the route tree in `main.rs`.

**Why:** This is an oversight from development, not an intentional design choice. The functionality can be enabled by adding the route to the read routes group.

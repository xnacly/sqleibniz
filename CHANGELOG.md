# Changelog

## v0.0.4

### Added

- Add semantic diagnostics for duplicate table, view, and virtual table declarations.
- Add semantic diagnostics for unknown relations in SELECT, INSERT, UPDATE, DELETE, JOIN, and INSERT ... SELECT statements.
- Add semantic diagnostics for unknown columns in SELECT projections, WHERE clauses, qualified column references, INSERT column lists, and UPDATE assignments.
- Add relation analysis infrastructure for tracking schemas, aliases, CTEs, and scoped table sources across statement analysis.
- Add dedicated analysis helpers for relation context, field extraction, virtual tables, and trace-analysis plumbing.
- Add new rule identifiers for `sqlite/duplicate-relation`, `sqlite/unknown-relation`, and `sqlite/unknown-column`.

### Changed

- Mark SELECT, INSERT, UPDATE, DELETE, CREATE VIEW, and CREATE VIRTUAL TABLE semantic analysis as supported in the README matrix.
- Speed up keyword suggestion lookup by bounding Levenshtein distance checks.
- Split contributor and local development guidance out of the README into `CONTRIBUTING.md`.
- Remove the old Makefile in favor of direct Cargo commands documented in contributor guidance.

### Fixed

- Fix the executable statement support example so `example/stmt.sql` passes again.
- Avoid reporting unknown relation diagnostics for standalone SELECTs, unresolved CTE internals, and other cases where the relation source cannot be known reliably.
- Avoid reporting unknown column diagnostics for ambiguous multi-table unqualified references and sources without known column metadata.

## v0.0.3-pre

### Added

- Add SARIF output support for diagnostics.
- Add Lua hook execution in the CLI and optional hook execution in LSP diagnostics.
- Add LSP configuration support for disabled rules.
- Add AST analysis modules for SQLite PRAGMAs, CREATE statements, and column declarations.
- Add diagnostics for unknown, deprecated, unsupported, and unsafe SQLite PRAGMA usage.
- Add recommendations for STRICT tables and diagnostics for nullable primary keys.
- Expand parser coverage for CREATE statements, CREATE VIRTUAL TABLE, SELECT, INSERT, UPDATE, DELETE, and expression operators.
- Add a dedicated `trace-analysis` feature.

### Changed

- Improve LSP diagnostic robustness, including malformed message handling, per-document state isolation, and non-empty diagnostic ranges.
- Summarize CLI diagnostics by rule in human-readable output.
- Refresh README examples, support tables, and configuration documentation.
- Increase identifier size checking to 52 characters.

### Fixed

- Fix SARIF reporting behavior and add integration coverage.
- Skip expected statement tokens when running hooks.

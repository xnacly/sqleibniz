# Changelog

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

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

### Added

- Initial DST-safe bucketing CLI with `bucket`, `range`, and `explain` commands
- Golden test harness with `UPDATE_GOLDEN=1` support
- CI workflow (Ubuntu, macOS, Windows)
- Release workflow with cross-compiled binaries
- Hardened CLI error handling (typed exit codes and JSON error envelopes in JSON mode)
- Regression tests for range end-exclusivity and nonexistent-local-midnight panic prevention

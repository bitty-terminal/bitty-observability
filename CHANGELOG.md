# Changelog

All notable changes to bitty-observability will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial observability API definitions
- `MetricsCollector` trait for counters, gauges, histograms
- `Logger` trait for structured logging with levels and context
- `Tracer` trait for distributed tracing spans
- Zero-dependency API crate (`bitty-observability-api`)
- Quality gates: fmt, clippy, tests, supply-chain audit
- CI: Linux, Windows, macOS, MSRV 1.85 checks

## [0.0.1] - 2026-10-02

Initial release with API definitions only. Implementation pending.

[Unreleased]: https://github.com/bitty-terminal/bitty-observability/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/bitty-terminal/bitty-observability/releases/tag/v0.0.1

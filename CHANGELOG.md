# Changelog

All notable changes to bitty-observability will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Aligned the API and core crates with the accepted Core observability contract
  (bitty-docs `W-71`, ADR-0015 Boundary 1):
  - `ObservableEvent` is now the bounded `Observation` record with typed
    `AttributeValue`s and optional `Attribution`;
  - `ObservabilityCapability` drops the control scope; `debug.inspect` and
    `debug.trace` remain;
  - `TraceConfig` defaults to disabled (opt-in) and carries the seam bounds;
  - `TraceBuffer` takes explicit `BufferLimits` and reports drops, truncations,
    and rejections through `DropReport`;
  - `redact_payload` keeps its allowlist behavior alongside new emission-time
    redaction (`redact_observation`, sensitive-by-default `Sensitivity`).
- `bitty-observability-api` is now zero-dependency; all workspace crates use
  only the standard library.

### Added

- `ObservationSink` read-only sink trait and bounded, detachable `Subscription`.
- Default-deny `AuthorizationGate` with separate inspect/subscribe grants and
  out-of-process consent.
- Fail-closed contract `ContractVersion` / `ContractRange` negotiation.
- Quality gates: fmt, clippy, tests, supply-chain audit
- CI: Linux, Windows, macOS, MSRV 1.85 checks

### Fixed

- `just supply-chain` now runs `cargo deny check` (auto-discovers `deny.toml`
  at the repo root); the previous `cargo deny check --config deny.toml`
  argument order is rejected by cargo-deny 0.20.2.

## [0.0.1] - 2026-10-02

Initial release with API definitions only. Implementation pending.

[Unreleased]: https://github.com/bitty-terminal/bitty-observability/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/bitty-terminal/bitty-observability/releases/tag/v0.0.1

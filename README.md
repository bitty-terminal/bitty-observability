# bitty-observability

Bitty L1 Rust Core Extension: the read-only observation seam for the terminal
runtime and plugin ecosystem. This repository holds the optional debug and trace
implementation side of observability; it holds no authority.

## Status

**Pre-1.0, API and shared implementation only.** All three crates are version
`0.0.1` and provide no stability guarantee before the `0.1.0` release.

This repository is **not consumed by `bitty` (Bitty Core) or by any other Bitty
repository**. The Core observation seam and its authorization gate, redaction
rules, and bounds are retained by Core (bitty-docs `W-71`); establishing them in
the host is the separate `W-100` task. Nothing here is integrated with Core, and
this repository makes no Core-integration claim. Runtime integration lives
outside this repository; host adoption is the separate `W-100` task.

## Architecture

```text
bitty-observability-api    # Pure types and traits (zero dependencies)
    |
bitty-observability-core   # Bounded buffer, filtering, redaction
    |
bitty-observability        # Facade crate re-exporting both layers
```

### Crates

- **`bitty-observability-api`** — zero-dependency contract types and traits:
  - `Observation` (candidate seam record): a bounded kind, a bounded attribute
    set, and optional `Attribution` to a subsystem or plugin;
  - `ObservationSink`: a read-only receiver with no control, mutation, or
    handle-passing method;
  - `Subscription`: a bounded, versioned, detachable handle, negotiated through
    `ContractRange` so an incompatible observer fails closed;
  - `ObservabilityCapability` / `AuthorizationGate`: the default-deny gate with
    separately granted `debug.inspect` and `debug.trace` scopes;
  - `TraceConfig` / `TraceRecord`: the opt-in trace configuration and the trace
    record, which is the same bounded `Observation` shape.
- **`bitty-observability-core`** — shared implementation that honors the Core
  bounds: `TraceBuffer` with fixed record, in-flight, and total in-memory
  limits plus explicit drop/truncation reporting; `EventFilter`
  (exact/prefix pattern matching); `redact_observation` and `redact_payload`
  for emission-time redaction.
- **`bitty-observability`** — facade crate re-exporting the API and core
  layers under a single dependency.

### Observation seam contract

The host side of the seam is the `ObservationHost` trait
(`bitty-observability-api`): what Core would push (`emit` of one bounded
`Observation` per boundary event, redacted at emission), what a consumer reads
(`drain_to` into its `ObservationSink`, with `DropReport` loss reporting and
`on_detached` teardown), and the bounds (record construction bounds,
per-subscription in-flight limit, drop-oldest discipline, inert with no
observer, fail-closed version attach through `subscribe` plus admission
re-check). The `crates/bitty-observability/tests/seam_conformance.rs` suite
proves the contract against a fake host, producer, and sink: fail-closed
attach, redaction at emission, explicit oldest-first drops, ordered delivery,
detach notification, and inert behavior.

### Network-counter disposition

The dead Core network counters (`MetricsSnapshot::network_rx_bytes` /
`network_tx_bytes`, sampled by `SystemMetricsService`) are **dropped
Core-side, not moved here**:

- no producer exists: only the test fake implements `MetricsAdapter`, and
  nothing feeds `StatusInputs::network_summary`, so the status `network`
  module only renders missing data;
- wrong layer: the counters sample the operating system, not a Core-owned
  mechanism, while the `W-71` seam observes Core mechanisms and this workspace
  is zero-dependency with no filesystem or network access;
- parked policy: metrics aggregation and export stay parked to `W-110` behind
  the opt-in gate, so moving the counters now would pre-empt that design.

This repository therefore provides no operating-system metrics sampling. If
`W-110` later designs an opt-in metrics pipeline, it emits bounded
`Observation` records through this same seam instead of reviving raw counters.
The Core-side removal is a Core follow-up referenced from #1629.

## Accepted-contract alignment

The repository tracks the accepted Core observability contract (bitty-docs
`docs/development/observability-boundary.md`, `W-71`) and ADR-0015 Boundary 1:

- **Read-only seam.** The API exposes no PTY, GPU, window, raw-input, or
  capability handle and no mutation method. A record carries only a bounded
  kind, bounded attributes, and optional attribution.
- **Default-deny gate.** `AuthorizationGate` starts closed; `debug.inspect` and
  `debug.trace` are granted independently; an out-of-process observer also needs
  explicit consent. `debug.control` is not an observability capability.
- **Redaction at emission.** Attributes are sensitive by default; only
  explicitly classified fields keep their value, everything else is replaced
  with `AttributeValue::Redacted`. Raw PTY bytes, clipboard content, raw
  environment data, and raw input (`DEFAULT_DENIED_FIELDS`) are never captured,
  even when allowlisted.
- **Bounds and explicit loss.** `BufferLimits` fixes a maximum record size,
  records in flight, and a total in-memory budget. Records drop
  oldest-first, truncation is marked on the record, and every loss is surfaced
  through `DropReport`. An inert buffer (no observer) stores nothing, and no
  persistence, file writer, network exporter, or metrics pipeline exists here.
- **Fail-closed versioning.** An observer attaches only on a non-empty
  `ContractRange` intersection; there is no best-effort attach.
- **No runtime privileges.** The workspace has no external dependencies, no
  network behavior, and no filesystem persistence.

### Version notes

- Workspace crates stay `0.0.1` (DIR-019); no compatibility promise is made
  before `0.1.0`.
- The observation schema version is separate from the crate version and is
  advertised through `ContractVersion` / `ContractRange`. Adding an ignorable
  record kind or attribute advances the minor version; a rename, removal, or
  changed meaning advances the major version and fails a mismatched attach.

## Usage

```toml
[dependencies]
bitty-observability = "0.0.1"
```

```rust
use bitty_observability::{
    AttributeValue, AuthorizationGate, BufferLimits, EventFilter, EventKind, Observation,
    ObservabilityCapability, ObserverOrigin, TraceBuffer,
};

let filter = EventFilter::new("terminal.*");
assert!(filter.matches("terminal.opened"));

let observation = Observation::new(EventKind::new("terminal.opened"), 1)
    .with_attribute("columns", AttributeValue::Unsigned(80));

let mut gate = AuthorizationGate::default_deny();
gate.grant(ObservabilityCapability::DebugTrace);
assert!(
    gate.authorize(ObservabilityCapability::DebugTrace, ObserverOrigin::InProcess)
        .is_ok()
);

let mut buffer = TraceBuffer::new(BufferLimits::default());
buffer.push(observation);
```

## Development

### Prerequisites

- Rust 1.85+ (MSRV)
- `just` 1.58.0+

### Quality Gates

```bash
just check    # fmt-check + clippy + tests
just test
just supply-chain
```

### CI

All changes must pass:

- Quality gates (fmt, clippy, tests)
- MSRV 1.85 check
- Linux, Windows, macOS builds
- Supply chain audit

## Open points

The following remain parked by the accepted contract and are not decided here;
they do not block the current alignment:

- exact trait, type, method, field, and capability-token spellings (owned by
  `W-100` and the security corpus);
- concrete bound values and default enabled/disabled posture beyond the
  documented defaults (`W-100`);
- exporter, collector, OTLP or other wire encoding, provider composition,
  sampling, and retention policy (`W-110`, behind the opt-in gate);
- whether the DevTools `debug.trace` path consumes this seam, without changing
  the accepted DevTools contract (`W-110` and the DevTools owner).

Core-integration evidence cannot be produced from this repository alone: no
Core or first-party caller consumes these crates, host adoption of the retained
seam is the separate `W-100` task, and the accepted contract's removal gates
(including the caller audit) remain unsatisfied by design.

## License

MIT OR Apache-2.0

## Related Projects

- [bitty](https://github.com/bitty-terminal/bitty) — Bitty terminal core
- [bitty-ipc](https://github.com/bitty-terminal/bitty-ipc) — IPC layer
  (independent repository since CTX-1585)

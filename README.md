# bitty-observability

Bitty L1 Rust Core Extension: observability API contracts for event tracing,
filtering, and payload redaction across the terminal runtime and plugin
ecosystem.

## Architecture

```text
bitty-observability-api    # Pure types and traits (zero runtime dependencies)
    ↑
bitty-observability-core   # Shared implementation (filters, buffers, redaction)
    ↑
bitty-observability        # Facade crate re-exporting both layers
```

### Crates

- **`bitty-observability-api`** — Pure types with zero implementation
  dependencies: `ObservabilityCapability` (debug.inspect/trace/control),
  `EventKind`/`ObservableEvent`, `TraceConfig`/`TraceRecord`. This is the only
  crate plugins and cross-repo consumers depend on for API contracts.
- **`bitty-observability-core`** — Shared implementation: `EventFilter`
  (exact/prefix pattern matching), `TraceBuffer` (bounded ring buffer with
  drop counting), `redact_payload` (allowlist-based field redaction).
- **`bitty-observability`** — Facade crate re-exporting the API and core
  layers under a single dependency.

## Status

**Pre-1.0, API and shared implementation only.** `bitty-ipc` has already been
extracted from Bitty Core as an independent repository
([bitty-terminal/bitty-ipc](https://github.com/bitty-terminal/bitty-ipc),
CTX-1585); this repository is not yet consumed by `bitty-ipc`, `bitty`, or any
other Bitty repository. Runtime integration (wiring these types into the
terminal core or a plugin) lives outside this repository and is not yet
scheduled.

## Usage

```toml
[dependencies]
bitty-observability = "0.0.1"
```

```rust
use bitty_observability::{EventFilter, EventKind, ObservableEvent, TraceBuffer};

let filter = EventFilter::new("terminal.*");
assert!(filter.matches("terminal.opened"));

let event = ObservableEvent::new(EventKind::new("terminal.opened"), 1);
let mut buffer = TraceBuffer::new(1000);
```

## Development

### Prerequisites

- Rust 1.85+ (MSRV)
- `just` 1.58.0+

### Quality Gates

```bash
just check    # Run all quality gates
just fmt-check
just clippy
just test
```

### CI

All changes must pass:

- Quality gates (fmt, clippy, tests)
- MSRV 1.85 check
- Linux, Windows, macOS builds
- Supply chain audit

## License

MIT

## Related Projects

- [bitty](https://github.com/bitty-terminal/bitty) — Bitty terminal core
- [bitty-ipc](https://github.com/bitty-terminal/bitty-ipc) — IPC layer
  (independent repository since CTX-1585)

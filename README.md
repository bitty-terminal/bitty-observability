# bitty-observability

**Observability API definitions for the Bitty terminal platform.**

This repository provides the foundational API contracts for observability capabilities in Bitty, supporting metrics collection, structured logging, and distributed tracing across the terminal runtime and plugin ecosystem.

## Repository Structure

- **`bitty-observability-api`**: Core API definitions and traits (zero runtime dependencies)
- **`bitty-observability`**: Main crate re-exporting all APIs

## Status

**Pre-alpha** — API definitions only. Implementation pending bitty-ipc extraction (tracked in bitty-terminal Core).

## Features

- **Metrics API**: Counter, gauge, histogram abstractions for performance monitoring
- **Logging API**: Structured logging with levels, context, and filtering
- **Tracing API**: Distributed tracing spans for request/operation tracking
- **Zero Dependencies**: API crate has no runtime dependencies for maximum compatibility

## Usage

```toml
[dependencies]
bitty-observability = "0.0.1"
```

```rust
use bitty_observability::{MetricsCollector, LogLevel};

// Metrics
collector.increment_counter("plugin.loaded", 1);
collector.record_histogram("request.latency_ms", duration_ms);

// Logging
log(LogLevel::Info, "plugin.lifecycle", "Plugin activated");
```

## Development

### Prerequisites

- Rust 1.85+ (MSRV)
- just 1.58.0+

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
- [bitty-ipc](https://github.com/bitty-terminal/bitty-ipc) — IPC layer (pending extraction)

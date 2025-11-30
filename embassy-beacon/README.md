# Embassy Beacon

**Embassy Beacon** is the firmware instrumentation component of **Embassy Watchtower**, an unofficial, community-driven task profiler, visualizer, and debugger for Embassy projects.

This crate is designed to run on your microcontroller alongside the **Embassy Executor**. It intercepts internal trace events and transmits them to the host PC via **defmt**, allowing for real-time monitoring.

## Getting Started

Follow these steps to instrument your firmware.

### 1. Add the Dependency
Add the crate to your Embassy project:

```shell
cargo add embassy-beacon
```

### 2. Enable Executor Tracing

The standard Embassy Executor does not expose internal events by default. You must enable the trace feature flag in your Cargo.toml to allow Beacon to hook into the execution loop:
```TOML
[dependencies]
embassy-executor = { version = "?.?.?", features = [..., "trace"] }
```

### 3. Initialize the Beacon

In your main application code (e.g., main.rs), simply import the crate. This triggers the necessary side effects to register the global event listeners automatically:
```Rust
use embassy_beacon as _;
```

### Next Steps

Once Embassy Beacon is integrated, running ```cargo run``` will result in your logs being flooded with raw trace messages. This is expected behavior.
To make sense of this data, you should use Embassy Visor on your PC. The Visor consumes these raw logs to provide a clean, visualized analysis of your tasks.
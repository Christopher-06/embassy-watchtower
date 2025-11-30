# Embassy Watchtower

**Embassy Watchtower** is an unofficial task profiler, visualizer, and debugger designed specifically for **Embassy-based** Embedded Rust projects.

It operates as a dual-component system to provide real-time insight into your microcontroller's operations:
* **Embassy Beacon:** The probe component running directly on your microcontroller.
* **Embassy Visor:** The host-side application running on your PC that visualizes the data.

Together, they allow you to monitor and analyze asynchronous tasks as they execute.

![Embassy Visor Screenshot](ressources/embassy-visor-screenshot-esp32-dual.png)

> **Compatibility Note:** This project has currently been tested on **ESP32** and **STM32** platforms.

## Getting Started

Follow these steps to integrate Watchtower into your project.

### 1. Add the Dependency
Add the `embassy-beacon` crate to your embedded project:

```shell
cargo add embassy-beacon
```

### 2. Enable Tracing

You must enable the trace feature in the Embassy Executor to allow Watchtower to hook into task events. Update your Cargo.toml:
```TOML
[dependencies]
embassy-executor = { version = "?.?.?", features = [..., "trace"] }
```

###  3. Initialize the Beacon

Import the crate in your main.rs (or lib.rs) to register the global event listeners. This ensures the beacon starts capturing data immediately.
Rust

```rust
use embassy_beacon as _;
```

### 4. Launch the Visor

Run the host application to start analyzing the incoming data streams and visualizing your task logs:
Shell

```shell
embassy-visor
```

## Roadmap & Features

This project is actively evolving. Below is the current status of features and future goals.

### Implemented

- [x] Core Task Tracing functionality

- [x] Multi-architecture support (ESP32, RP2040, STM32)

- [x] Human-readable names for Tasks and Executors (extracted via ELF parsing)

- [x] Terminal User Interface (TUI) for real-time visualization
- [x] Support for Multi-Priority Executors (Thread mode vs. Interrupt mode)

- [x] Dual-Core Microcontroller support

### Planned

- [ ] Advanced Analytics (e.g., Task Runtime, Wait/Poll times) via GUI?

- [ ] Extended Documentation and Examples

- [ ] Unit and Integration Tests

- [ ] File Persistence (Saving trace data to disk)

- [ ] Expanded Testing/Examples for RP2040 and STM32

## Work In Progress

>  Note: This project is currently under active development.

Community feedback is highly appreciated! If you encounter bugs or have feature requests, please feel free to open an Issue or submit a Pull Request.
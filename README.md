# Proxy Wallet Service

## Overview

`proxy-wallet-service` is a service that provides the same client operations as linera-service without installing the entire linera binaries. It abstracts the linera service implementation and exposes a clean API for:

- **Creating wallets**
- **Managing Chains**

The service is designed to be lightweight, highly concurrent, and easy to integrate with other services in a distributed system.
(current binary_size: 8.0 MB)

## Capabilities

- **Client abstraction** – Handles communication.
- **Coming soon...**

## Getting Started

### Command Usage

The service provides three CLI commands via `cargo run`:

- **Metrics**

  ```bash
  cargo run -- metrics
  ```

  Prints resource metrics and starts the resource logger.

- **Deploy**

  ```bash
  cargo run -- deploy --path <PROJECT_PATH> [--json-argument <JSON>]
  ```

  Deploys an application located at `<PROJECT_PATH>`. Optionally provide a JSON‑encoded initialization argument.

- **Watch**
  ```bash
  cargo run -- watch --app-id <APP_ID>
  ```
  Subscribes to an existing application identified by `<APP_ID>` and watches for events.

These commands correspond to the subcommands defined in `src/main.rs`. Use the `--help` flag for more details:

```bash
cargo run -- --help
```

### Command Details

- **Metrics**: Retrieves and prints resource metrics, and starts the resource logger.
- **Deploy**: Deploys an application. Provide the path to the project directory containing the contract and service WASM files. Optionally pass a JSON‑encoded initialization argument.
- **Watch**: Subscribes to an existing application by its ID and watches for events.

### Prerequisites

- Rust toolchain (stable) – see `rust-toolchain.toml`
- Cargo (installed with Rust)

### Build with Folder Structure

```bash
\_root
  |_linera-protocol
  |_proxy-wallet-service

cd proxy-wallet-service
cargo build --release
```

### Run

```bash
cargo run
```

### Testing

<!-- Run the unit and integration tests: -->

Planning to have tests in future.

## Project Structure

```
src/
├── main.rs        # Entry point – starts the service
├── client.rs      # Client abstraction for wallet communication
├── resource.rs    # Resource Usage Metrics i.e, cpu, mem
└── wallet.rs      # Wallet implementations
Cargo.toml          # Project metadata and dependencies
README.md
```

## Contributing

Contributions are welcome! Please fork the repository, create a feature branch, and submit a pull request. Follow the existing code style and run `cargo fmt` before committing.

## License

This project is licensed under the Apache License – see the `LICENSE` file for details.

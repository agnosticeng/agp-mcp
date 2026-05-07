# agp-mcp 🦀

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-1.6.0-blue.svg)](https://modelcontextprotocol.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance **Model Context Protocol (MCP)** server for **ClickHouse**, written in Rust. This server enables AI models (like Claude) to interact with ClickHouse databases securely and efficiently by retrieving schemas and executing read-only SQL queries.

## ✨ Features

- 🔍 **Schema Discovery**: Automatically fetch table names and their creation queries.
- ⚡ **Read-Only Queries**: Execute SQL queries via a secure AGP API proxy.
- 🌐 **Dual Transport**: Supports both **stdio** (standard for CLI usage) and **HTTP/SSE** (standard for web-based clients).
- 🔓 **CORS Support**: Permissive CORS policy for the HTTP transport to enable integration with various web-based MCP clients.
- 🛠️ **Idiomatic Rust**: Built with the latest Rust 2024 edition, featuring structured error handling and comprehensive documentation.
- 🛡️ **Secure**: Designed for read-only access to protect your data integrity.

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)
- A ClickHouse instance accessible via an AGP API proxy.

### Installation

```bash
git clone https://github.com/agnosticeng/agp-mcp.git
cd agp-mcp
cargo build --release
```

### Usage

The server supports two transport modes: **stdio** (default) and **HTTP**.

#### STDIO Mode (Default)
Best for use with local MCP clients like Claude Desktop.

```bash
# Using CLI argument
./target/release/agp-mcp --url "https://your-clickhouse-proxy.com"

# Using environment variable
export PROXY_URL="https://your-clickhouse-proxy.com"
./target/release/agp-mcp
```

#### HTTP Mode
Best for remote integration or web-based clients.

```bash
./target/release/agp-mcp --url "https://your-clickhouse-proxy.com" --http
```

By default, the HTTP server binds to `127.0.0.1:8001`. You can customize this using an environment variable:

```bash
export HTTP_BIND_ADDRESS="0.0.0.0:8080"
./target/release/agp-mcp --http
```

## 🛠️ MCP Tools

Once connected, the following tools are available to the AI:

1. **`get_schema`**: Retrieves the list of tables and their `CREATE TABLE` statements from the `system.tables` table.
2. **`execute_query`**: Executes a SQL query. The request expects a `query` string and returns the data in a structured JSON format.

## ⚙️ Configuration

| Argument | Environment Variable | Description |
|----------|----------------------|-------------|
| `--url`  | `PROXY_URL`          | **Required**. The URL of the ClickHouse AGP proxy. |
| `--http` | -                    | Enables the HTTP/SSE transport mode. |
| -        | `HTTP_BIND_ADDRESS`  | The address and port to bind the HTTP server to (Default: `127.0.0.1:8001`). |

## 🧪 Development

### Running Tests

We maintain a robust test suite covering unit tests and end-to-end scenarios.

```bash
# Run Rust unit tests
cargo test

# Run Python E2E tests (requires the binary to be built)
python3 tests/e2e.py
```

### Linting & Formatting

```bash
cargo clippy
cargo fmt
```

## 🤝 Contributing

Contributions are welcome! If you have an idea for a new feature or found a bug, please open an issue or submit a pull request.

1. Fork the project.
2. Create your feature branch (`git checkout -b feature/AmazingFeature`).
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4. Push to the branch (`git push origin feature/AmazingFeature`).
5. Open a Pull Request.

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

---

Built with ❤️ by the [Agnostic Engineering](https://github.com/agnosticeng) team.

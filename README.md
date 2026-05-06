# agp-mcp 🦀

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-1.0.0-blue.svg)](https://modelcontextprotocol.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance **Model Context Protocol (MCP)** server for **ClickHouse**, written in Rust. This server enables AI models (like Claude) to interact with ClickHouse databases securely and efficiently by retrieving schemas and executing read-only SQL queries.

## ✨ Features

- 🔍 **Schema Discovery**: Automatically fetch table names and their creation queries.
- ⚡ **Read-Only Queries**: Execute SQL queries via a secure AGP API proxy.
- 🦀 **Rust-Powered**: Built with the latest Rust 2024 edition for safety and speed.
- 🛡️ **Secure**: Designed for read-only access to protect your data integrity.
- 📊 **Rich Metadata**: Returns meta-information about columns, rows, and execution statistics.

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

Run the server by providing the URL to your ClickHouse proxy:

```bash
# Using CLI argument
./target/release/agp-mcp --url "https://your-clickhouse-proxy.com"

# Using environment variable
export PROXY_URL="https://your-clickhouse-proxy.com"
./target/release/agp-mcp
```

## 🛠️ MCP Tools

Once connected, the following tools are available to the AI:

1. **`get_schema`**: Retrieves the list of tables and their `CREATE TABLE` statements from the `system.tables` table.
2. **`execute_query`**: Executes a SQL query. The request expects a `query` string and returns the data in a structured JSON format.

## ⚙️ Configuration

| Argument | Environment Variable | Description |
|----------|----------------------|-------------|
| `--url`  | `PROXY_URL`          | **Required**. The URL of the ClickHouse AGP proxy. |

## 🧪 Development

### Running Tests

We maintain high test coverage to ensure reliability.

```bash
cargo test
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

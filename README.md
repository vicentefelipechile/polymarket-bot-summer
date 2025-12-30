# PoC
Never tested and never used in production or development.

# Polymarket HFT Autonomous Trading System

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-frequency trading bot for Polymarket prediction markets, leveraging the `polymarket-hft` crate for activity spike exploitation with an intuitive async CLI interface.

## ⚠️ Disclaimer

**This is a real-money trading bot. Trading involves substantial risk of loss. Only trade with funds you can afford to lose. This software is provided "as is" without warranty of any kind.**

## 🎯 Features

- **Activity Spike Detection**: Identifies trading opportunities using volume velocity and order book imbalance algorithms
- **High-Frequency Execution**: Built on Rust's async runtime for minimal latency
- **SQLite Persistence**: WAL-mode database for high-concurrency data storage
- **User-Friendly CLI**: Interactive REPL with colored output and command history
- **Smart Onboarding**: Helpful error messages guide you through setup
- **Portfolio Tracking**: Real-time P&L monitoring and position management

## 📋 Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- A Polymarket account with API access
- Polygon wallet with USDC

## 🚀 Quick Start

### 1. Clone & Build

```bash
git clone <repository-url>
cd polymarket-bot-summer
cargo build --release
```

### 2. Configure Environment

Copy the example environment file and fill in your credentials:

```bash
cp .env.example .env
```

Edit `.env` with your actual values:

```env
# Get this from your wallet (MetaMask → Account Details → Export Private Key)
POLYMARKET_PK=0x...your_private_key...

# Get these from https://polymarket.com → Settings → API Keys
CLOB_API_KEY=your_api_key_here
CLOB_API_SECRET=your_api_secret_here
CLOB_PASSPHRASE=your_passphrase_here
```

### 3. Run the Bot

```bash
cargo run --release
```

The bot will automatically:
- ✓ Validate your configuration
- ✓ Initialize the database
- ✓ Start the interactive CLI

## 📖 Usage

Once running, you'll see an interactive prompt:

```
polymarket>
```

### Information Commands

| Command | Description |
|---------|-------------|
| `/help` | Show all available commands |
| `/currentstate` | Display system health, WebSocket status, and latency |
| `/lastbid` | Show details of the last order placed |
| `/balance` | Display current USDC balance and portfolio value |
| `/active` | List all currently open orders |
| `/markets` | Show monitored market IDs |
| `/pnl` | Display realized vs unrealized profit & loss |

### Control Commands

| Command | Description |
|---------|-------------|
| `/pause` | Pause new order placement (cancel-only mode) |
| `/resume` | Resume normal trading operations |
| `/panic` | 🚨 **EMERGENCY**: Cancel all orders immediately and pause |
| `/export` | Export session log to CSV |
| `/exit` | Shut down the bot |

## 🏗️ Architecture

### Core Modules

- **Data Ingestion**: Subscribes to CLOB WebSocket, RTDS, and Gamma feeds via `polymarket-hft`
- **Spike Detection**: Implements volume velocity and order book imbalance algorithms
- **Execution Engine**: Manages order placement, cancellation, and portfolio tracking
- **Database Layer**: SQLite with WAL mode for concurrent read/write operations
- **CLI Interface**: Async REPL built with `rustyline` and `colored` for great UX

### Spike Detection Algorithms

1. **Volume Velocity**
   ```
   V_v = ΔVolume / Δt
   ```
   Triggers when velocity exceeds configured threshold

2. **Order Book Imbalance (OBI)**
   ```
   OBI = (V_bids - V_asks) / (V_bids + V_asks)
   ```
   Range: -1 (all asks) to +1 (all bids)

## ⚙️ Configuration

All configuration is done via environment variables in `.env`:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POLYMARKET_PK` | ✅ | - | Your Polygon wallet private key |
| `CLOB_API_KEY` | ✅ | - | Polymarket API key |
| `CLOB_API_SECRET` | ✅ | - | Polymarket API secret |
| `CLOB_PASSPHRASE` | ✅ | - | Polymarket API passphrase |
| `MAX_ORDER_SIZE` | ❌ | 100 | Maximum order size in USDC |
| `MIN_ORDER_SIZE` | ❌ | 1 | Minimum order size in USDC |
| `VOLUME_VELOCITY_THRESHOLD` | ❌ | 1000 | Velocity threshold for spike detection |
| `OBI_THRESHOLD` | ❌ | 0.3 | Order book imbalance threshold |
| `DATABASE_PATH` | ❌ | ./bot_history.db | SQLite database file path |
| `RUST_LOG` | ❌ | info | Log level (trace/debug/info/warn/error) |

## 🗄️ Database Schema

The bot maintains a SQLite database with the following tables:

- `markets`: Market metadata
- `trades`: Historical trade data
- `orders`: Order lifecycle tracking
- `orderbook_snapshots`: Order book state over time
- `volume_velocity_events`: Detected spike events
- `portfolio_snapshots`: Portfolio value history

## 🔒 Security

- **Never commit your `.env` file** - it contains sensitive credentials
- Private keys are handled using the `secrecy` crate
- Database files are gitignored by default
- Always use separate API keys for testing vs production

## 🛠️ Development

### Running Tests

```bash
cargo test
```

### Checking Code

```bash
cargo check
cargo clippy
```

### Building for Production

```bash
cargo build --release
```

The optimized binary will be in `target/release/polymarket-hft-bot`.

## 🚧 Roadmap

- [x] Phase 1: Core infrastructure and CLI
- [ ] Phase 2: Full `polymarket-hft` integration
- [ ] Phase 3: Advanced trading strategies
- [ ] Phase 4: Performance profiling and optimization
- [ ] Phase 5: Web dashboard for monitoring

## 📝 License

MIT License - See [LICENSE](LICENSE) file for details

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📧 Support

For issues and questions, please open an issue on GitHub.

---

**Remember**: This bot trades with real money. Always start with small amounts and understand the risks involved in automated trading.

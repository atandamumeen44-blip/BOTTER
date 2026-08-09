
# 🦈 Sharklet v2

AI-powered cross-chain flash loan arbitrage bot.

## Features

- Multi-chain support (Polygon, Arbitrum, Base, Amoy)
- Flash loan arbitrage via Aave V3
- Multi-execution loop (20+ trades per opportunity)
- Auto-reinvesting gas budget
- Enterprise risk engine (circuit breaker, adaptive limits)
- Full pre-flight simulation
- Live dashboard (health / metrics)
- SQLite trade journal

## Quick Start

### 1. Clone & Build

```bash
git clone https://github.com/yourusername/sharklet.git
cd sharklet
cp .env.example .env
# Edit .env with your keys
cargo build --release

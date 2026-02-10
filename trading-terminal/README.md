# tx-terminal

A modern, terminal-based trading interface for Solana tokens. 

![Demo](https://via.placeholder.com/800x400?text=tx-terminal+Demo)

## Features

- **Real-time Market Data**: View price, volume, and market cap updates.
- **"Trenches" View**: Monitor new, bonding, and migrated tokens in a dense, card-based layout.
- **Interactive Swaps**: Execute SOL swaps directly from the terminal using Jupiter Aggregator.
- **Chart Visualization**: ASCII-based candlestick charts with auto-scrolling.
- **Fast & Lightweight**: Built with Rust and Ratatui for maximum performance.

## Installation

### From Crates.io

```bash
cargo install tx-terminal
```

### From Source

```bash
git clone https://github.com/anurag/trading-terminal.git
cd trading-terminal/trading-terminal
cargo install --path .
```

## Usage

Simply run the `tx` command:

```bash
tx
```

### Key Bindings

- **Arrow Keys**: Navigate between columns and tokens.
- **Enter**: View token details.
- **Esc**: Go back / Exit.
- **S**: Initiate a swap (in Token Details view).
- **Type Numbers**: Enter swap amount.

## Configuration

The application authenticates using your local Solana wallet. Ensure you have a keypair at `~/.config/solana/id.json` or configure the path via environment variables (future feature).

## License

MIT

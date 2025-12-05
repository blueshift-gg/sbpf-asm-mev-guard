# sbpf-asm-mev-guard

Protect your Solana transactions from MEV attacks by identifying and avoiding blacklisted validators.

## What It Does

This tool generates a bitmap that marks which slots in an epoch are controlled by validators known to perform sandwich attacks. Your program can use this bitmap to skip sending transactions during those slots, protecting you from MEV extraction.

Created with [sbpf](https://github.com/blueshift-gg/sbpf)

## Quick Start

### 1. Generate the Blocklist

Fetch validators with sandwich attack rates ≥ 1%:

```bash
cargo run --bin blocklist
```

Output: `blocklist.txt` with validator addresses

### 2. Generate the Bitmap

Create a bitmap marking blocklisted validator slots for the current epoch:

```bash
cargo run --bin bitmap
```

Output: `epoch-<N>-bitmap.bin` where each bit represents whether that slot's leader is blocklisted (1) or safe (0)

## How It Protects You

1. **Blocklist** Queries the Sandwiched.me API to identify validators with high sandwich attack rates in the last 30 days
2. **Bitmap** Samples the epoch's leader schedule (every 4th slot) and creates a bitmap marking blocklisted leaders
3. Your program reads the bitmap to **skip transactions during dangerous slots**, avoiding validators that may sandwich your trades

The bitmap samples every 4th slot (one per leader rotation) - for a 432k slot epoch, this creates a 13.5KB bitmap that's efficient to check on-chain.
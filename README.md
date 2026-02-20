# 🚀 Stellar Raise Contracts

A **crowdfunding smart contract** built on the [Stellar](https://stellar.org/) network using [Soroban](https://soroban.stellar.org/).

## Overview

Stellar Raise lets anyone create a crowdfunding campaign on-chain. Contributors pledge tokens toward a goal before a deadline. If the goal is met, the creator withdraws the funds. If not, contributors are refunded automatically.

### Key Features

| Feature | Description |
| :--- | :--- |
| **Initialize** | Create a campaign with a goal, deadline, and token |
| **Contribute** | Pledge tokens before the deadline |
| **Withdraw** | Creator claims funds after a successful campaign |
| **Refund** | Contributors individually reclaim tokens if the goal is missed (pull-based) |

## Project Structure

```text
stellar-raise-contracts/
├── .github/workflows/rust_ci.yml   # CI pipeline
├── contracts/crowdfund/
│   ├── src/
│   │   ├── lib.rs                  # Smart contract logic
│   │   └── test.rs                 # Unit tests
│   └── Cargo.toml                  # Contract dependencies
├── Cargo.toml                      # Workspace config
├── CONTRIBUTING.md
├── README.md
└── LICENSE
```

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- The `wasm32-unknown-unknown` target:

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- [Stellar CLI](https://soroban.stellar.org/docs/getting-started/setup) (optional, for deployment)

## Getting Started

```bash
# Clone the repo
git clone https://github.com/<your-org>/stellar-raise-contracts.git
cd stellar-raise-contracts

# Build the contract
cargo build --release --target wasm32-unknown-unknown

# Run tests
cargo test --workspace
```

## Contract Interface

```rust
// Create a new campaign
fn initialize(env, creator, token, goal, deadline, min_contribution);

// Pledge tokens to the campaign
fn contribute(env, contributor, amount);

// Creator withdraws after successful campaign
fn withdraw(env);

// Individual contributor claims refund if goal not met (pull-based)
fn refund_single(env, contributor);

// View functions
fn total_raised(env) -> i128;
fn goal(env) -> i128;
fn deadline(env) -> u64;
fn contribution(env, contributor) -> i128;
fn min_contribution(env) -> i128;
```

## Pull-based Refund Model

This contract uses a **pull-based refund** pattern for scalability and gas efficiency.

### Why Pull-based?

A traditional "push" refund (where one transaction refunds all contributors) would:
- Fail with thousands of contributors due to resource limits
- Be expensive and unpredictable in cost
- Create a single point of failure

### How it Works

If the campaign goal is **not met** by the deadline:
1. Each contributor must claim their own refund by calling `refund_single`
2. Contributors can claim at any time after the deadline
3. The refund is processed immediately and securely

### Example: Claiming Your Refund

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source <YOUR_SECRET_KEY> \
  -- refund_single \
  --contributor <YOUR_ADDRESS>
```

## Deployment (Testnet)

```bash
# Build the optimized WASM
cargo build --release --target wasm32-unknown-unknown

# Deploy using Stellar CLI
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/crowdfund.wasm \
  --network testnet \
  --source <YOUR_SECRET_KEY>
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a full history of notable changes.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

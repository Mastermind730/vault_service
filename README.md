# Collateral Vault Management System

A comprehensive, production-ready vault management system for a decentralized perpetual futures exchange built on Solana. This system securely manages user collateral (USDT) in program-controlled vaults and enables cross-program invocations for trading operations.

## 🎯 Overview

This system provides:
- **Secure Collateral Management**: Non-custodial vault system using Solana PDAs
- **Real-time Balance Tracking**: Monitor and reconcile vault balances
- **Cross-Program Support**: Enable position managers to lock/unlock collateral
- **High Performance**: Support for 1000+ concurrent users
- **Comprehensive Monitoring**: Balance snapshots, audit logs, and TVL tracking

## 🏗️ Architecture

### Components

1. **Anchor Smart Contract** (`programs/vault-manager/`)
   - Collateral vault program with PDA-based account management
   - SPL Token integration for USDT transfers
   - Cross-program invocation support

2. **Rust Backend Service** (`src/`)
   - Vault lifecycle management
   - Real-time balance tracking and reconciliation
   - REST API and WebSocket support
   - MongoDB integration for transaction history

### System Flow

```
User Wallet
    ↓
[Deposit] → Vault PDA → [Lock Collateral] → Position Manager
    ↑            ↓
[Withdraw] ← [Unlock Collateral] ← Position Closed
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+
- Solana CLI 1.17+
- Anchor 0.29+
- MongoDB 4.4+
- Node.js 16+ (for testing)

### Installation

1. **Clone the repository**
```bash
cd vault-manager-service
```

2. **Set up environment variables**
```bash
cp .env.example .env
```

**IMPORTANT**: Edit `.env` and set your MongoDB URI:
```
MONGODB_URI=mongodb://your-connection-string-here
```

3. **Build the Anchor program**
```bash
anchor build
```

4. **Deploy the program (local/devnet)**
```bash
# Start local validator
solana-test-validator

# In another terminal
anchor deploy
```

5. **Build the backend service**
```bash
cargo build --release
```

6. **Run the service**
```bash
cargo run --release
```

## 📡 API Documentation

### Base URL
```
http://localhost:8080
```

### Endpoints

#### Vault Operations

**Initialize Vault**
```http
POST /vault/initialize
Content-Type: application/json

{
  "user_pubkey": "User's Solana public key"
}
```

**Get Vault Balance**
```http
GET /vault/balance/{vault_pubkey}
```

**Get Vault by Owner**
```http
GET /vault/owner/{owner_pubkey}
```

**Record Deposit**
```http
POST /vault/deposit
Content-Type: application/json

{
  "user_pubkey": "User's Solana public key",
  "amount": 1000000  // Amount in smallest units
}
```

**Record Withdrawal**
```http
POST /vault/withdraw
Content-Type: application/json

{
  "user_pubkey": "User's Solana public key",
  "amount": 500000
}
```

**Get Transaction History**
```http
GET /vault/transactions/{vault_pubkey}?limit=50
```

#### Internal Operations (Position Manager)

**Lock Collateral**
```http
POST /internal/lock
Content-Type: application/json

{
  "vault_pubkey": "Vault PDA address",
  "amount": 100000
}
```

**Unlock Collateral**
```http
POST /internal/unlock
Content-Type: application/json

{
  "vault_pubkey": "Vault PDA address",
  "amount": 100000
}
```

#### Analytics

**Get TVL**
```http
GET /analytics/tvl
```

**Health Check**
```http
GET /health
```

### WebSocket Connection

Connect to real-time updates:
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Update:', data);
};
```

**Message Types:**
- `balance_update`: Vault balance changed
- `deposit`: New deposit processed
- `withdrawal`: Withdrawal processed
- `lock`: Collateral locked
- `unlock`: Collateral unlocked
- `tvl_update`: Total Value Locked updated

## 🔒 Security Features

1. **PDA-based Vaults**: Program-controlled accounts prevent unauthorized access
2. **Authority Validation**: Only authorized programs can lock/unlock collateral
3. **Balance Verification**: All operations verify sufficient balance
4. **Audit Logging**: Complete audit trail of all vault operations
5. **Reconciliation**: Periodic balance reconciliation with on-chain state

## 📊 Database Schema

### Collections

**vaults**
- Stores vault account information
- Indexes: `owner`, `_id`

**transactions**
- Transaction history for all vaults
- Indexes: `vault + timestamp`, `signature`

**balance_snapshots**
- Hourly and daily balance snapshots
- Indexes: `vault + timestamp`

**audit_logs**
- Security audit trail
- Indexes: `timestamp`

**tvl_stats**
- Total Value Locked statistics
- Indexes: `timestamp`

## 🧪 Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
anchor test
```

### Program Tests
```bash
cd programs/vault-manager
cargo test-bpf
```

## 📈 Monitoring

The system includes comprehensive monitoring:

1. **Balance Tracking**: Real-time balance monitoring and reconciliation
2. **Snapshots**: Hourly and daily balance snapshots
3. **TVL Updates**: Periodic Total Value Locked calculations
4. **Audit Logs**: Complete audit trail
5. **Unusual Activity Detection**: Alerts on suspicious transactions

## 🔧 Configuration

All configuration is managed through environment variables (see `.env.example`):

- **Solana**: RPC URL, commitment level
- **MongoDB**: Connection string and database name
- **Server**: Host and port
- **Vault Program**: Program ID and USDT mint address

## 🏃 Development

### Project Structure

```
vault-manager-service/
├── programs/
│   └── vault-manager/       # Anchor smart contract
│       ├── src/
│       │   ├── lib.rs       # Main program
│       │   ├── state.rs     # Account structures
│       │   └── errors.rs    # Error definitions
│       └── Cargo.toml
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── database.rs          # MongoDB operations
│   ├── vault_manager.rs     # Vault lifecycle management
│   ├── balance_tracker.rs   # Balance monitoring
│   ├── transaction_builder.rs # Transaction construction
│   ├── models.rs            # Data models
│   ├── errors.rs            # Error types
│   ├── api/
│   │   ├── mod.rs          # API router
│   │   └── handlers.rs     # API handlers
│   └── websocket.rs         # WebSocket handler
├── Anchor.toml
├── Cargo.toml
└── .env.example
```

### Adding New Features

1. Update the Anchor program in `programs/vault-manager/src/lib.rs`
2. Add corresponding backend logic in `src/vault_manager.rs`
3. Create API endpoints in `src/api/handlers.rs`
4. Update models in `src/models.rs`

## 🚨 Important Notes

### MongoDB Setup

**You MUST configure MongoDB before running the service!**

1. Install MongoDB locally or use MongoDB Atlas
2. Update the `MONGODB_URI` in your `.env` file
3. The service will automatically create the database and collections

Example MongoDB URIs:
```
# Local MongoDB
MONGODB_URI=mongodb://localhost:27017

# MongoDB Atlas
MONGODB_URI=mongodb+srv://username:password@cluster.mongodb.net/

# With authentication
MONGODB_URI=mongodb://username:password@localhost:27017
```

### USDT Mint Address

Update `USDT_MINT` in `.env` with your USDT token mint address:
- **Mainnet**: Check Solana token list
- **Devnet**: Create a test token or use existing devnet USDT
- **Localnet**: Create your own test token

## 🎬 Demo Video Requirements

When creating your demonstration video (10-15 minutes):

1. **Architecture Overview** (3 min)
   - Explain the vault system design
   - Show PDA derivation
   - Explain CPI flow

2. **Live Demo** (5 min)
   - Initialize a vault
   - Deposit collateral
   - Lock/unlock collateral
   - Withdraw funds
   - Show real-time WebSocket updates

3. **Code Walkthrough** (5 min)
   - Anchor program structure
   - SPL Token CPI implementation
   - Balance tracking mechanism
   - Security measures

4. **Q&A** (2 min)
   - Address potential concerns
   - Discuss scalability
   - Future enhancements

## 📝 License

This project is for educational and demonstration purposes.

## 🤝 Contributing

This is a demonstration project. For production use, additional security audits and testing are recommended.

## 📞 Support

For questions or issues, please refer to the documentation or create an issue in the repository.

---

**Built with ❤️ using Rust, Solana, and Anchor**
# vault_service
#   v a u l t _ s e r v i c e  
 
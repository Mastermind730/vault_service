# System Architecture

## Overview

The Collateral Vault Management System is a comprehensive solution for managing user collateral in a decentralized perpetual futures exchange built on Solana.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         User Layer                          │
├─────────────────────────────────────────────────────────────┤
│  Web App  │  Mobile App  │  Trading Terminal  │  API Client│
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                      API Gateway                            │
├─────────────────────────────────────────────────────────────┤
│  REST API (Axum)  │  WebSocket (Real-time Updates)        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Backend Services (Rust)                   │
├─────────────────────────────────────────────────────────────┤
│  VaultManager  │  BalanceTracker  │  TransactionBuilder   │
└────────────┬────────────┬──────────────┬────────────────────┘
             │            │              │
             ▼            ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Storage & State                           │
├─────────────────────────────────────────────────────────────┤
│  MongoDB         │  Solana Blockchain  │  Cache (Future)   │
└─────────────────────────────────────────────────────────────┘
```

## Component Architecture

### 1. Solana Smart Contract (Anchor Program)

**Location:** `programs/vault-manager/`

#### Account Structure

```
┌──────────────────────────────────────┐
│      CollateralVault (PDA)           │
├──────────────────────────────────────┤
│  owner: Pubkey                       │
│  token_account: Pubkey               │
│  total_balance: u64                  │
│  locked_balance: u64                 │
│  available_balance: u64              │
│  total_deposited: u64                │
│  total_withdrawn: u64                │
│  created_at: i64                     │
│  last_updated: i64                   │
│  bump: u8                            │
└──────────────────────────────────────┘
```

#### PDA Derivation

```rust
// Vault PDA
seeds = [b"vault", user.key().as_ref()]

// Authority PDA
seeds = [b"authority"]
```

#### Instruction Flow

```
Initialize Vault
    ↓
Create PDA Account
    ↓
Create Associated Token Account
    ↓
Set Initial State
    ↓
Return Vault Address
```

### 2. Backend Service Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Main Service                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────┐ │
│  │ VaultManager   │  │BalanceTracker │  │ Transaction  │ │
│  │                │  │                │  │  Builder     │ │
│  │ - Initialize   │  │ - Monitor      │  │ - Build TX   │ │
│  │ - Deposit      │  │ - Reconcile    │  │ - Send TX    │ │
│  │ - Withdraw     │  │ - Snapshot     │  │ - Confirm    │ │
│  │ - Lock/Unlock  │  │ - Alert        │  │              │ │
│  └────────────────┘  └────────────────┘  └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3. Data Flow

#### Deposit Flow

```
1. User initiates deposit
   ↓
2. Frontend creates transaction
   ↓
3. User signs transaction
   ↓
4. Transaction sent to Solana
   ↓
5. Anchor program executes:
   - Transfer SPL tokens via CPI
   - Update vault balances
   - Emit deposit event
   ↓
6. Backend service:
   - Detects event/receives API call
   - Updates MongoDB
   - Creates snapshot
   - Broadcasts WebSocket update
```

#### Lock/Unlock Flow (CPI)

```
Position Manager Program
   ↓
CPI Call to Vault Program
   ↓
Vault Program Validates:
   - Caller is authorized
   - Sufficient balance
   ↓
Update vault state:
   - Adjust locked_balance
   - Adjust available_balance
   ↓
Emit lock/unlock event
   ↓
Backend updates database
```

### 4. Security Model

#### Access Control

```
┌─────────────────────────────────────┐
│         Vault Operations            │
├─────────────────────────────────────┤
│  Owner Only:                        │
│  - Initialize vault                 │
│  - Deposit                          │
│  - Withdraw                         │
│                                     │
│  Authorized Programs Only:          │
│  - Lock collateral                  │
│  - Unlock collateral                │
│  - Transfer collateral              │
│                                     │
│  Admin Only:                        │
│  - Add authorized program           │
│  - Remove authorized program        │
└─────────────────────────────────────┘
```

#### Security Checks

1. **PDA Ownership**: Vault PDAs are owned by the program
2. **Signer Validation**: Operations require appropriate signers
3. **Balance Verification**: All operations check sufficient balance
4. **Authority Checks**: Lock/unlock verify caller authorization
5. **Overflow Protection**: Use checked arithmetic

### 5. Database Schema

```
MongoDB Database: vault_manager

Collections:
┌─────────────────────────────────────┐
│  vaults                             │
│  - _id (vault address)              │
│  - owner                            │
│  - balances                         │
│  - statistics                       │
├─────────────────────────────────────┤
│  transactions                       │
│  - _id (uuid)                       │
│  - vault                            │
│  - type, amount, signature          │
│  - timestamp, status                │
├─────────────────────────────────────┤
│  balance_snapshots                  │
│  - _id (uuid)                       │
│  - vault, balances                  │
│  - timestamp, type                  │
├─────────────────────────────────────┤
│  audit_logs                         │
│  - _id (uuid)                       │
│  - action, user, vault              │
│  - timestamp, success               │
├─────────────────────────────────────┤
│  tvl_stats                          │
│  - _id (uuid)                       │
│  - totals, vault_count              │
│  - timestamp                        │
└─────────────────────────────────────┘
```

### 6. Monitoring & Observability

```
┌──────────────────────────────────────────────┐
│          Monitoring Components               │
├──────────────────────────────────────────────┤
│                                              │
│  Balance Tracker:                            │
│  - Hourly snapshots (every 1 hour)          │
│  - Daily snapshots (every 24 hours)         │
│  - Reconciliation (every 5 minutes)         │
│                                              │
│  TVL Tracker:                                │
│  - Calculate TVL (every 1 minute)           │
│  - Broadcast updates via WebSocket          │
│                                              │
│  Audit Logger:                               │
│  - Log all operations                       │
│  - Track success/failure                    │
│  - Record IP addresses                      │
│                                              │
└──────────────────────────────────────────────┘
```

### 7. Scalability Considerations

#### Horizontal Scaling

```
Load Balancer
     ↓
┌────┴────┬────────┬────────┐
│ API 1   │ API 2  │ API 3  │
└────┬────┴────┬───┴────┬───┘
     └─────────┼────────┘
               ↓
        MongoDB Cluster
```

#### Performance Optimizations

1. **Connection Pooling**: MongoDB connection pool
2. **Caching**: Redis for frequently accessed data (future)
3. **Batch Processing**: Batch snapshot creation
4. **Async Operations**: Non-blocking I/O throughout
5. **WebSocket Broadcasting**: Efficient multi-client updates

### 8. Deployment Architecture

#### Development
```
Local Machine
├── Solana Test Validator
├── MongoDB (Docker)
└── Backend Service
```

#### Staging/Production
```
┌─────────────────────────────────────┐
│         Load Balancer               │
└─────────────┬───────────────────────┘
              ↓
┌─────────────────────────────────────┐
│      API Instances (K8s Pods)       │
└─────────────┬───────────────────────┘
              ↓
┌─────────────┴───────────────────────┐
│  MongoDB Atlas (Managed)            │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  Solana Mainnet/Devnet              │
└─────────────────────────────────────┘
```

## Technology Stack

### Smart Contract
- **Language**: Rust
- **Framework**: Anchor 0.29
- **Blockchain**: Solana
- **Token Standard**: SPL Token

### Backend Service
- **Language**: Rust
- **Web Framework**: Axum
- **Async Runtime**: Tokio
- **Database**: MongoDB
- **WebSocket**: tokio-tungstenite

### Testing
- **Anchor Tests**: TypeScript + Mocha
- **Unit Tests**: Rust cargo test
- **Integration Tests**: Anchor test suite

## Security Considerations

1. **Private Key Management**: Never store private keys in code
2. **Rate Limiting**: Implement in production
3. **Input Validation**: Validate all user inputs
4. **Audit Logging**: Log all operations
5. **Access Control**: Restrict internal endpoints
6. **HTTPS**: Use TLS in production
7. **CORS**: Configure properly for production

## Future Enhancements

1. **Redis Caching**: For improved performance
2. **GraphQL API**: Alternative to REST
3. **Multi-signature Vaults**: Enhanced security
4. **Yield Integration**: Generate yield on idle collateral
5. **Advanced Analytics**: Machine learning for fraud detection
6. **Mobile SDK**: Native mobile libraries

---

For implementation details, see the [README.md](../README.md) and [API.md](API.md).

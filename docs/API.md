# API Documentation

## Vault Manager Service API Reference

Base URL: `http://localhost:8080`

---

## Authentication

Currently, the API does not require authentication for demonstration purposes. In production, implement:
- API key authentication
- JWT tokens
- Rate limiting
- IP whitelisting for internal endpoints

---

## Endpoints

### Health Check

#### GET `/health`

Check if the service is running.

**Response:**
```
200 OK
```

---

### Vault Operations

#### POST `/vault/initialize`

Initialize a new vault for a user.

**Request Body:**
```json
{
  "user_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
}
```

**Response:**
```json
{
  "signature": "vault_pda_address",
  "status": "success"
}
```

**Status Codes:**
- `200`: Success
- `400`: Invalid request
- `500`: Internal server error

---

#### GET `/vault/balance/:vault`

Get vault balance by vault public key.

**Parameters:**
- `vault` (path): Vault PDA address

**Response:**
```json
{
  "vault": "vault_pda_address",
  "owner": "owner_pubkey",
  "total_balance": 1000000000,
  "locked_balance": 300000000,
  "available_balance": 700000000,
  "total_deposited": 1500000000,
  "total_withdrawn": 500000000
}
```

**Status Codes:**
- `200`: Success
- `404`: Vault not found
- `500`: Internal server error

---

#### GET `/vault/owner/:owner`

Get vault balance by owner public key.

**Parameters:**
- `owner` (path): Owner's Solana public key

**Response:**
Same as `/vault/balance/:vault`

---

#### POST `/vault/deposit`

Record a deposit transaction (called after on-chain transaction).

**Request Body:**
```json
{
  "user_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "amount": 1000000000
}
```

**Response:**
```json
{
  "signature": "transaction_signature",
  "status": "confirmed"
}
```

**Status Codes:**
- `200`: Success
- `400`: Invalid amount
- `404`: Vault not found
- `500`: Internal server error

---

#### POST `/vault/withdraw`

Record a withdrawal transaction.

**Request Body:**
```json
{
  "user_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "amount": 500000000
}
```

**Response:**
```json
{
  "signature": "transaction_signature",
  "status": "confirmed"
}
```

**Status Codes:**
- `200`: Success
- `400`: Insufficient balance or invalid amount
- `404`: Vault not found
- `500`: Internal server error

---

#### GET `/vault/transactions/:vault`

Get transaction history for a vault.

**Parameters:**
- `vault` (path): Vault PDA address
- `limit` (query, optional): Number of transactions to return (default: 50)

**Example:**
```
GET /vault/transactions/vault_address?limit=100
```

**Response:**
```json
[
  {
    "id": "transaction_id",
    "vault": "vault_address",
    "transaction_type": "deposit",
    "amount": 1000000000,
    "signature": "tx_signature",
    "timestamp": "2024-01-15T10:30:00Z",
    "status": "confirmed",
    "error_message": null
  }
]
```

**Status Codes:**
- `200`: Success
- `404`: Vault not found
- `500`: Internal server error

---

### Internal Operations

⚠️ **These endpoints should be protected in production and only accessible to authorized programs.**

#### POST `/internal/lock`

Lock collateral for a position (called by position manager).

**Request Body:**
```json
{
  "vault_pubkey": "vault_pda_address",
  "amount": 300000000
}
```

**Response:**
```json
{
  "signature": "lock_success",
  "status": "confirmed"
}
```

**Status Codes:**
- `200`: Success
- `400`: Insufficient available balance
- `404`: Vault not found
- `500`: Internal server error

---

#### POST `/internal/unlock`

Unlock collateral when position is closed.

**Request Body:**
```json
{
  "vault_pubkey": "vault_pda_address",
  "amount": 300000000
}
```

**Response:**
```json
{
  "signature": "unlock_success",
  "status": "confirmed"
}
```

**Status Codes:**
- `200`: Success
- `400`: Invalid unlock amount
- `404`: Vault not found
- `500`: Internal server error

---

### Analytics

#### GET `/analytics/tvl`

Get Total Value Locked statistics.

**Response:**
```json
{
  "total_tvl": 5000000000000,
  "total_locked": 1500000000000,
  "total_available": 3500000000000,
  "vault_count": 1234,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Status Codes:**
- `200`: Success
- `500`: Internal server error

---

## WebSocket API

### Connection

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');
```

### Message Types

#### Balance Update
```json
{
  "type": "balance_update",
  "vault": "vault_address",
  "total_balance": 1000000000,
  "locked_balance": 300000000,
  "available_balance": 700000000
}
```

#### Deposit Event
```json
{
  "type": "deposit",
  "vault": "vault_address",
  "amount": 1000000000,
  "signature": "tx_signature"
}
```

#### Withdrawal Event
```json
{
  "type": "withdrawal",
  "vault": "vault_address",
  "amount": 500000000,
  "signature": "tx_signature"
}
```

#### Lock Event
```json
{
  "type": "lock",
  "vault": "vault_address",
  "amount": 300000000
}
```

#### Unlock Event
```json
{
  "type": "unlock",
  "vault": "vault_address",
  "amount": 300000000
}
```

#### TVL Update
```json
{
  "type": "tvl_update",
  "total_tvl": 5000000000000,
  "vault_count": 1234
}
```

#### Error Message
```json
{
  "type": "error",
  "message": "Error description"
}
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "HTTP_STATUS_CODE",
  "message": "Detailed error message"
}
```

### Common Error Codes

- `400 BAD REQUEST`: Invalid input parameters
- `401 UNAUTHORIZED`: Authentication required
- `404 NOT FOUND`: Resource not found
- `500 INTERNAL SERVER ERROR`: Server error

---

## Rate Limiting

In production, implement rate limiting:
- 100 requests per minute per IP for public endpoints
- 1000 requests per minute for internal endpoints
- WebSocket: 10 connections per IP

---

## Best Practices

1. **Always verify transaction signatures on-chain**
2. **Use WebSocket for real-time updates instead of polling**
3. **Implement exponential backoff for retries**
4. **Cache vault balances locally when possible**
5. **Monitor WebSocket connection health**

---

## Example Usage

### JavaScript/TypeScript

```typescript
import axios from 'axios';

const API_BASE = 'http://localhost:8080';

// Initialize vault
async function initializeVault(userPubkey: string) {
  const response = await axios.post(`${API_BASE}/vault/initialize`, {
    user_pubkey: userPubkey
  });
  return response.data;
}

// Get balance
async function getBalance(vaultAddress: string) {
  const response = await axios.get(`${API_BASE}/vault/balance/${vaultAddress}`);
  return response.data;
}

// WebSocket connection
const ws = new WebSocket('ws://localhost:8080/ws');
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Update:', data);
};
```

### Rust

```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Initialize vault
    let response = client
        .post("http://localhost:8080/vault/initialize")
        .json(&json!({
            "user_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
        }))
        .send()
        .await?;
    
    println!("{:?}", response.text().await?);
    
    Ok(())
}
```

---

For more information, see the main [README.md](../README.md).

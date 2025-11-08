# Security Best Practices

## Overview

This document outlines critical security considerations for the Vault Manager System.

---

## Smart Contract Security

### 1. PDA Security

**Best Practices:**
- ✅ Use deterministic PDA derivation
- ✅ Validate PDA ownership in all instructions
- ✅ Include bump seed in account state
- ❌ Never allow user-provided PDA seeds

**Example:**
```rust
#[account(
    mut,
    seeds = [b"vault", user.key().as_ref()],
    bump = vault.bump,
)]
pub vault: Account<'info, CollateralVault>,
```

### 2. Authority Validation

**Critical Checks:**
- Owner verification for deposits/withdrawals
- Authority validation for lock/unlock operations
- Admin-only for authority management

```rust
#[account(
    mut,
    has_one = owner @ VaultError::UnauthorizedOwner
)]
pub vault: Account<'info, CollateralVault>,
```

### 3. Arithmetic Safety

**Use Checked Operations:**
```rust
// ✅ Good
vault.total_balance = vault.total_balance
    .checked_add(amount)
    .ok_or(VaultError::NumericalOverflow)?;

// ❌ Bad
vault.total_balance += amount; // Can overflow!
```

### 4. Reentrancy Protection

**Solana's Account Model:**
- Single-threaded execution prevents classic reentrancy
- Still validate account state before and after CPIs

### 5. Access Control

```rust
// Authorized programs list
pub struct VaultAuthority {
    pub authorized_programs: Vec<Pubkey>,
    pub admin: Pubkey,
}

// Verify caller is authorized
require!(
    authority.authorized_programs.contains(&caller_program_id),
    VaultError::UnauthorizedProgram
);
```

---

## Backend Service Security

### 1. Environment Variables

**Never commit sensitive data:**
```bash
# .env (gitignored)
MONGODB_URI=mongodb+srv://user:pass@cluster.mongodb.net/
SOLANA_RPC_URL=https://your-private-rpc.com
```

**Use secrets management in production:**
- AWS Secrets Manager
- HashiCorp Vault
- Kubernetes Secrets

### 2. Input Validation

```rust
// Validate all user inputs
pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DepositRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    // Validate amount
    if payload.amount == 0 {
        return Err(VaultServiceError::InvalidAmount("Amount must be > 0".to_string()));
    }
    
    // Validate pubkey format
    let user_pubkey = Pubkey::from_str(&payload.user_pubkey)
        .map_err(|e| VaultServiceError::InvalidPublicKey(e))?;
    
    // ... rest of logic
}
```

### 3. Rate Limiting

**Implement rate limiting middleware:**
```rust
// Example with tower-governor
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap(),
);

let app = Router::new()
    .layer(GovernorLayer {
        config: Box::leak(governor_conf),
    });
```

### 4. HTTPS/TLS

**Always use TLS in production:**
```nginx
server {
    listen 443 ssl http2;
    server_name api.yourdomain.com;
    
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
    
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
}
```

### 5. CORS Configuration

```rust
// Restrict origins in production
let cors = CorsLayer::new()
    .allow_origin("https://yourdomain.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE, AUTHORIZATION]);
```

---

## Database Security

### 1. Connection Security

**Use SSL/TLS for MongoDB:**
```
mongodb+srv://user:pass@cluster.mongodb.net/?ssl=true&authSource=admin
```

### 2. Access Control

**Principle of Least Privilege:**
```javascript
// MongoDB user with minimal permissions
db.createUser({
  user: "vault_service",
  pwd: "strong_password",
  roles: [
    { role: "readWrite", db: "vault_manager" }
  ]
});
```

### 3. Query Injection Prevention

**Always use parameterized queries:**
```rust
// ✅ Good - uses BSON doc macro
collection.find_one(doc! { "_id": vault_pubkey }, None).await?

// ❌ Bad - string interpolation (if it were possible)
// Never build queries from user input directly
```

### 4. Data Encryption

**Encrypt sensitive data at rest:**
- Use MongoDB encrypted storage engine
- Encrypt backups
- Use field-level encryption for PII

---

## API Security

### 1. Authentication

**Implement API key authentication:**
```rust
async fn authenticate(
    headers: HeaderMap,
) -> Result<(), VaultServiceError> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(VaultServiceError::Unauthorized)?;
    
    // Validate API key
    validate_api_key(api_key)?;
    
    Ok(())
}
```

### 2. Internal Endpoints

**Restrict access to internal endpoints:**
```rust
// IP whitelist for internal operations
const ALLOWED_IPS: &[&str] = &["127.0.0.1", "10.0.0.0/8"];

async fn verify_internal_access(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<(), VaultServiceError> {
    if !is_allowed_ip(&addr.ip()) {
        return Err(VaultServiceError::Unauthorized);
    }
    Ok(())
}
```

### 3. Request Validation

**Validate all request data:**
```rust
#[derive(Deserialize, Validate)]
pub struct DepositRequest {
    #[validate(length(min = 32, max = 44))]
    pub user_pubkey: String,
    
    #[validate(range(min = 1, max = 1_000_000_000_000))]
    pub amount: u64,
}
```

---

## Operational Security

### 1. Key Management

**Never expose private keys:**
- Store keypairs securely (HSM in production)
- Use different keys for different environments
- Implement key rotation
- Use multi-signature for critical operations

### 2. Monitoring & Alerting

**Alert on suspicious activity:**
```rust
// Example: Large withdrawal detection
if amount > LARGE_WITHDRAWAL_THRESHOLD {
    send_alert(&format!(
        "Large withdrawal detected: {} USDT from vault {}",
        amount, vault_pubkey
    )).await?;
}
```

### 3. Audit Logging

**Log all critical operations:**
```rust
pub async fn log_audit(
    &self,
    vault: Option<String>,
    user: Option<String>,
    action: String,
    details: serde_json::Value,
    ip_address: Option<String>,
    success: bool,
) -> Result<()> {
    let log = AuditLog {
        id: uuid::Uuid::new_v4().to_string(),
        vault,
        user,
        action,
        details,
        ip_address,
        timestamp: Utc::now(),
        success,
    };
    
    self.db.insert_audit_log(log).await?;
    Ok(())
}
```

### 4. Backup & Recovery

**Regular backups:**
```bash
#!/bin/bash
# backup-mongodb.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backup/mongodb/$DATE"

# Create backup
mongodump --uri="$MONGODB_URI" --out="$BACKUP_DIR"

# Encrypt backup
tar czf - "$BACKUP_DIR" | openssl enc -aes-256-cbc -salt -out "$BACKUP_DIR.tar.gz.enc"

# Remove unencrypted backup
rm -rf "$BACKUP_DIR"

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR.tar.gz.enc" "s3://backups/mongodb/"

# Keep only last 30 days
find /backup/mongodb -mtime +30 -delete
```

---

## Security Checklist

### Pre-Deployment

- [ ] All private keys secured
- [ ] Environment variables not committed
- [ ] SSL/TLS certificates configured
- [ ] Rate limiting enabled
- [ ] CORS properly configured
- [ ] Input validation on all endpoints
- [ ] Authentication/authorization implemented
- [ ] Audit logging enabled

### Smart Contract

- [ ] PDA derivation validated
- [ ] All arithmetic uses checked operations
- [ ] Authority checks on all instructions
- [ ] Access control properly enforced
- [ ] No hardcoded addresses
- [ ] Error messages don't leak sensitive info

### Infrastructure

- [ ] Firewall rules configured
- [ ] Database access restricted
- [ ] Backups automated and tested
- [ ] Monitoring and alerting set up
- [ ] Secrets management configured
- [ ] Network isolated (if applicable)

### Ongoing

- [ ] Regular security audits
- [ ] Dependency updates
- [ ] Log review
- [ ] Penetration testing
- [ ] Incident response plan
- [ ] Disaster recovery tested

---

## Incident Response

### 1. Detection

Monitor for:
- Unusual transaction patterns
- Failed authentication attempts
- Balance discrepancies
- Service errors

### 2. Response

**Immediate actions:**
```bash
# Pause service if compromise suspected
sudo systemctl stop vault-manager

# Review logs
journalctl -u vault-manager -n 1000 > incident-logs.txt

# Check database for unauthorized changes
mongosh $MONGODB_URI --eval "db.audit_logs.find().sort({timestamp: -1}).limit(100)"
```

### 3. Recovery

- Restore from known good backup
- Patch vulnerability
- Rotate all credentials
- Notify affected users
- Document incident

---

## Security Contacts

**Report security vulnerabilities to:**
- Email: security@yourdomain.com
- Bug bounty program: (if applicable)

**Never disclose vulnerabilities publicly before reporting!**

---

## Additional Resources

- [Solana Security Best Practices](https://docs.solana.com/developing/programming-model/security)
- [Anchor Security Guidelines](https://www.anchor-lang.com/docs/security)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

---

**Security is an ongoing process, not a one-time task. Stay vigilant!**

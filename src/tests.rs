#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_balance_calculation() {
        let total_balance = 1000u64;
        let locked_balance = 300u64;
        let available_balance = total_balance - locked_balance;
        
        assert_eq!(available_balance, 700);
    }

    #[test]
    fn test_overflow_protection() {
        let balance = u64::MAX;
        let amount = 1u64;
        
        let result = balance.checked_add(amount);
        assert!(result.is_none());
    }

    #[test]
    fn test_underflow_protection() {
        let balance = 100u64;
        let amount = 200u64;
        
        let result = balance.checked_sub(amount);
        assert!(result.is_none());
    }
}

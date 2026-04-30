use rust_decimal::{Decimal, dec};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Struct representing an account record to be handled by the payment engine
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    pub const fn new(client: u16) -> Self {
        Self {
            client,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        }
    }

    pub const fn get_available(&self) -> Decimal {
        self.available
    }

    pub const fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.available += amount;
        self.total += amount;
    }

    pub fn withdraw(&mut self, amount: Decimal) {
        self.available -= amount;
        self.total -= amount;
    }

    pub fn dispute(&mut self, amount: Decimal) {
        self.held += amount;
        self.available -= amount;
    }

    pub fn resolve(&mut self, amount: Decimal) {
        self.held -= amount;
        self.available += amount;
    }

    pub fn chargeback(&mut self, amount: Decimal) {
        self.held -= amount;
        self.total -= amount;
    }

    pub const fn lock(&mut self) {
        self.locked = true;
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{},{:.4},{:.4},{:.4},{}`",
            self.client, self.available, self.held, self.total, self.locked
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_accounts_to_csv() {
        let expected = b"client,available,held,total,locked
1,0.0,0.0,0.0,false
2,0.0,0.0,0.0,false
";

        let accounts = vec![Account::new(1), Account::new(2)];
        let mut writer = csv::Writer::from_writer(Vec::new());
        for account in accounts {
            writer.serialize(account).unwrap();
        }
        let result = writer.into_inner().unwrap();
        assert_eq!(result, expected);
    }
}

use getset::{Getters, Setters};
use rust_decimal::{Decimal, dec};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

/// Struct representing an account record to be handled by the payment engine
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Getters, Setters)]
#[getset(get = "pub")]
pub struct Account {
    client: u16,
    #[serde(serialize_with = "serialize_amount_rounded_4dp")]
    available: Decimal,
    #[serde(serialize_with = "serialize_amount_rounded_4dp")]
    held: Decimal,
    #[serde(serialize_with = "serialize_amount_rounded_4dp")]
    total: Decimal,
    #[getset(set = "pub")]
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

fn serialize_amount_rounded_4dp<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{:.4}", value.round_dp(4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_accounts_to_csv() {
        let mut accounts = vec![Account::new(1), Account::new(2)];
        accounts[0].deposit(dec!(1.0));
        accounts[1].deposit(dec!(2.123499999999999)); // check for rounding

        let mut writer = csv::Writer::from_writer(Vec::new());
        for account in accounts {
            writer.serialize(account).unwrap();
        }

        let expected = b"client,available,held,total,locked
1,1.0000,0.0000,1.0000,false
2,2.1235,0.0000,2.1235,false
";
        let result = writer.into_inner().unwrap();
        assert_eq!(result, expected);
    }
}

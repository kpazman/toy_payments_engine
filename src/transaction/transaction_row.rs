use getset::Getters;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fmt;

use crate::transaction::TransactionType;

/// Struct representing a transaction row in the CSV files
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Getters)]
#[getset(get = "pub")]
pub(super) struct TransactionRow {
    r#type: TransactionType,
    client: u16,
    tx: u32,
    #[serde(deserialize_with = "deserialize_amount_rounded_4dp")]
    amount: Option<Decimal>,
}

fn deserialize_amount_rounded_4dp<'de, D>(d: D) -> Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let amount = Option::<Decimal>::deserialize(d)?;
    Ok(amount.map(|v| v.round_dp(4)))
}

impl fmt::Display for TransactionRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.amount {
            Some(amount) => write!(
                f,
                "`{},{},{},{:.4}`",
                self.r#type, self.client, self.tx, amount
            ),
            None => write!(f, "`{},{},{}`", self.r#type, self.client, self.tx),
        }
    }
}

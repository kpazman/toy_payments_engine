use getset::{Getters, Setters};
use rust_decimal::Decimal;
use std::fmt;

use crate::transaction::TransactionType;

/// Struct representing a Deposit/Withdrawal transaction that is to be recorded
#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub")]
pub struct TransactionRecord {
    r#type: TransactionType,
    client: u16,
    tx: u32,
    amount: Decimal,
    #[getset(set = "pub")]
    disputed: bool,
}

impl TransactionRecord {
    pub(super) const fn new(
        r#type: TransactionType,
        client: u16,
        tx: u32,
        amount: Decimal,
        disputed: bool,
    ) -> Self {
        Self {
            r#type,
            client,
            tx,
            amount,
            disputed,
        }
    }
}

impl fmt::Display for TransactionRecord {
    // Display the transaction in the same format as the input CSV file (disputed field is not included)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{},{},{},{:.4}`",
            self.r#type, self.client, self.tx, self.amount
        )
    }
}

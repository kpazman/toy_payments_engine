use getset::{Getters, Setters};
use std::fmt;

use crate::transaction::TransactionType;

/// Struct representing a Dispute/Resolve/Chargeback, that modifies an existing transaction
#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub")]
pub struct TransactionModifier {
    r#type: TransactionType,
    client: u16,
    tx: u32,
}

impl TransactionModifier {
    pub const fn new(r#type: TransactionType, client: u16, tx: u32) -> Self {
        Self { r#type, client, tx }
    }
}

impl fmt::Display for TransactionModifier {
    // Display the transaction in the same format as the input CSV file
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{},{},{}`", self.r#type, self.client, self.tx)
    }
}

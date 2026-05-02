mod transaction_modifier;
mod transaction_record;
mod transaction_row;
mod transaction_stream;

pub use transaction_modifier::TransactionModifier;
pub use transaction_record::TransactionRecord;
pub use transaction_stream::TransactionStream;

use getset::{Getters, Setters};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::fmt;
use thiserror::Error;

use crate::transaction::transaction_row::TransactionRow;

/// Type representing errors in parsed transaction validation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum TransactionError {
    #[error("Missing amount from transaction `{0},{1},{2}`")]
    MissingAmount(TransactionType, u16, u32),
    #[error("Unrequired amount for transaction `{0},{1},{2},{3}`")]
    UnrequiredAmount(TransactionType, u16, u32, Decimal),
}

/// Supported transaction types
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Struct representing a transaction to be handled by the payment engine
#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub")]
pub struct Transaction {
    r#type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
    #[getset(set = "pub")]
    disputed: bool,
}

impl Transaction {
    pub const fn new(
        r#type: TransactionType,
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
        disputed: bool,
    ) -> Result<Self, TransactionError> {
        let transaction = Self {
            r#type,
            client,
            tx,
            amount,
            disputed,
        };

        // validate the transaction: Deposit/Withdrawal must have amount, Dispute/Resolve/Chargeback must not have amount
        match transaction.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if transaction.amount.is_none() {
                    return Err(TransactionError::MissingAmount(
                        transaction.r#type,
                        transaction.client,
                        transaction.tx,
                    ));
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                if let Some(amount) = transaction.amount {
                    return Err(TransactionError::UnrequiredAmount(
                        transaction.r#type,
                        transaction.client,
                        transaction.tx,
                        amount,
                    ));
                }
            }
        }

        Ok(transaction)
    }

    pub const fn to_record(&self) -> Result<TransactionRecord, TransactionError> {
        if let Some(amount) = self.amount {
            Ok(TransactionRecord::new(
                self.r#type,
                self.client,
                self.tx,
                amount,
                self.disputed,
            ))
        } else {
            Err(TransactionError::MissingAmount(
                self.r#type,
                self.client,
                self.tx,
            ))
        }
    }

    pub const fn to_modifier(&self) -> Result<TransactionModifier, TransactionError> {
        if let Some(amount) = self.amount {
            Err(TransactionError::UnrequiredAmount(
                self.r#type,
                self.client,
                self.tx,
                amount,
            ))
        } else {
            Ok(TransactionModifier::new(self.r#type, self.client, self.tx))
        }
    }
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = TransactionError;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        Self::new(
            *row.r#type(),
            *row.client(),
            *row.tx(),
            *row.amount(),
            false,
        )
    }
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let row: TransactionRow = Deserialize::deserialize(deserializer)?;
        Self::try_from(row).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Transaction {
    // Display the transaction in the same format as the input CSV file (disputed field is not included)
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

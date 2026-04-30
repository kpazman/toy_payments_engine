use csv::DeserializeRecordsIntoIter;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, fmt, fs::File, io::Read, path::PathBuf};
use thiserror::Error;

/// Type representing errors in parsed transaction validation
#[derive(Error, Debug, PartialEq)]
enum TransactionError {
    #[error("Missing amount from transaction `{0}`")]
    MissingAmount(TransactionRow),
    #[error("Unrequired amount for transaction `{0}`")]
    UnrequiredAmount(TransactionRow),
}

/// Supported transaction types
#[derive(Debug, Deserialize, PartialEq, Clone, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Struct representing a transaction row in the CSV files
#[derive(Debug, Deserialize, PartialEq, Clone)]
struct TransactionRow {
    r#type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
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

impl TransactionRow {
    fn validate(self) -> Result<Self, TransactionError> {
        match self.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if self.amount.is_none() {
                    return Err(TransactionError::MissingAmount(self));
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                if self.amount.is_some() {
                    return Err(TransactionError::UnrequiredAmount(self));
                }
            }
        }

        Ok(self)
    }
}

/// Struct representing a transaction record to be handled by the payment engine
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
    pub disputed: bool,
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = TransactionError;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        let valid_row = row.validate()?;

        Ok(Transaction {
            r#type: valid_row.r#type,
            client: valid_row.client,
            tx: valid_row.tx,
            amount: valid_row.amount,
            disputed: false,
        })
    }
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let row: TransactionRow = Deserialize::deserialize(deserializer)?;
        Transaction::try_from(row).map_err(serde::de::Error::custom)
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

/// Iterator over transactions from a reader
pub struct TransactionStream<R: Read> {
    inner: DeserializeRecordsIntoIter<R, Transaction>,
}

impl<R: Read> TransactionStream<R> {
    /// Create a new TransactionStream from a reader
    pub fn from_reader(reader: R) -> Self {
        Self {
            inner: csv::ReaderBuilder::new()
                .flexible(true)
                .trim(csv::Trim::All)
                .from_reader(reader)
                .into_deserialize::<Transaction>(),
        }
    }
}

impl<R: Read> Iterator for TransactionStream<R> {
    type Item = Result<Transaction, csv::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl TransactionStream<File> {
    /// Create a new TransactionStream from a file
    pub fn from_file(file: &PathBuf) -> std::io::Result<Self> {
        let file = File::open(file);
        if let Ok(file) = file {
            return Ok(Self::from_reader(file));
        }

        let error = file.unwrap_err();
        log::error!("Failed to open file: {error}");
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn deserialize_correct_transactions() {
        // has leading or trailing whitespace for some fields, missing or included optional last field
        let csv = "type,client,tx,amount
deposit, 1, 1, 1
withdrawal,2,2 , 2.1234
dispute,1,3,
resolve,1,3
chargeback,1,3";

        let expected = vec![
            Transaction {
                r#type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(1.0)),
                disputed: false,
            },
            Transaction {
                r#type: TransactionType::Withdrawal,
                client: 2,
                tx: 2,
                amount: Some(dec!(2.1234)),
                disputed: false,
            },
            Transaction {
                r#type: TransactionType::Dispute,
                client: 1,
                tx: 3,
                amount: None,
                disputed: false,
            },
            Transaction {
                r#type: TransactionType::Resolve,
                client: 1,
                tx: 3,
                amount: None,
                disputed: false,
            },
            Transaction {
                r#type: TransactionType::Chargeback,
                client: 1,
                tx: 3,
                amount: None,
                disputed: false,
            },
        ];

        let records = TransactionStream::from_reader(csv.as_bytes())
            .collect::<Result<Vec<Transaction>, csv::Error>>()
            .unwrap();
        assert_eq!(records, expected);
    }

    #[test]
    fn deserialize_invalid_type() {
        let csv = "type,client,tx,amount
invalid,1,1,1.0";

        let result = TransactionStream::from_reader(csv.as_bytes())
            .collect::<Result<Vec<Transaction>, csv::Error>>();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown variant"));
    }

    #[test]
    fn deserialize_missing_amount() {
        let csv = "type,client,tx,amount
deposit,1,1";

        let result = TransactionStream::from_reader(csv.as_bytes())
            .collect::<Result<Vec<Transaction>, csv::Error>>();

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains(
                &TransactionError::MissingAmount(TransactionRow {
                    r#type: TransactionType::Deposit,
                    client: 1,
                    tx: 1,
                    amount: None,
                })
                .to_string()
            )
        );
    }

    #[test]
    fn deserialize_unrequired_amount() {
        let csv = "type,client,tx,amount
dispute,1,1,1.0";

        let result = TransactionStream::from_reader(csv.as_bytes())
            .collect::<Result<Vec<Transaction>, csv::Error>>();

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains(
                &TransactionError::UnrequiredAmount(TransactionRow {
                    r#type: TransactionType::Dispute,
                    client: 1,
                    tx: 1,
                    amount: Some(dec!(1.0)),
                })
                .to_string()
            )
        );
    }
}

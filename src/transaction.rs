use csv::DeserializeRecordsIntoIter;
use getset::{Getters, Setters};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, fmt, fs::File, io::Read, path::PathBuf};
use thiserror::Error;

/// Type representing errors in parsed transaction validation
#[derive(Error, Debug, PartialEq, Eq)]
pub enum TransactionError {
    #[error("Missing amount from transaction `{0},{1},{2}`")]
    MissingAmount(TransactionType, u16, u32),
    #[error("Unrequired amount for transaction `{0},{1},{2},{3}`")]
    UnrequiredAmount(TransactionType, u16, u32, Decimal),
}

/// Supported transaction types
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, strum::Display)]
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
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
struct TransactionRow {
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
}

impl TryFrom<TransactionRow> for Transaction {
    type Error = TransactionError;

    fn try_from(row: TransactionRow) -> Result<Self, Self::Error> {
        Self::new(row.r#type, row.client, row.tx, row.amount, false)
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
        // has leading or trailing whitespace for some fields, extra decimal places, missing or included optional last field
        let csv = "type,client,tx,amount
deposit, 1, 1, 1
withdrawal,2,2 , 2.1234
withdrawal,2,3 , 2.123499999999999
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
                r#type: TransactionType::Withdrawal,
                client: 2,
                tx: 3,
                amount: Some(dec!(2.1235)),
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
        assert!(result.unwrap_err().to_string().contains(
            &TransactionError::MissingAmount(TransactionType::Deposit, 1, 1).to_string()
        ));
    }

    #[test]
    fn deserialize_unrequired_amount() {
        let csv = "type,client,tx,amount
dispute,1,1,1.5";

        let result = TransactionStream::from_reader(csv.as_bytes())
            .collect::<Result<Vec<Transaction>, csv::Error>>();

        assert!(result.is_err());
        dbg!(result.as_ref().unwrap_err().to_string());
        dbg!(
            &TransactionError::UnrequiredAmount(TransactionType::Dispute, 1, 1, dec!(1.5))
                .to_string()
        );
        assert!(
            result.unwrap_err().to_string().contains(
                &TransactionError::UnrequiredAmount(TransactionType::Dispute, 1, 1, dec!(1.5))
                    .to_string()
            )
        );
    }
}

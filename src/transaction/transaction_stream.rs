use std::{fs::File, io::Read, path::PathBuf};

use csv::DeserializeRecordsIntoIter;

use crate::transaction::Transaction;

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
    use crate::transaction::{TransactionError, TransactionType};
    use rust_decimal::dec;

    #[test]
    fn deserialize_correct_transactions() {
        // has leading or trailing whitespace for some fields, extra decimal places, missing or included optional last field
        let csv = "type,client,tx,amount
deposit, 1, 1, 1
withdrawal,1,2 , 2.1234
withdrawal,1,3 , 2.123499999999999
dispute,1,3,
resolve,1,3
chargeback,1,3";

        let expected = vec![
            Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap(),
            Transaction::new(TransactionType::Withdrawal, 1, 2, Some(dec!(2.1234)), false).unwrap(),
            Transaction::new(TransactionType::Withdrawal, 1, 3, Some(dec!(2.1235)), false).unwrap(),
            Transaction::new(TransactionType::Dispute, 1, 3, None, false).unwrap(),
            Transaction::new(TransactionType::Resolve, 1, 3, None, false).unwrap(),
            Transaction::new(TransactionType::Chargeback, 1, 3, None, false).unwrap(),
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
        assert!(
            result.unwrap_err().to_string().contains(
                &TransactionError::UnrequiredAmount(TransactionType::Dispute, 1, 1, dec!(1.5))
                    .to_string()
            )
        );
    }
}

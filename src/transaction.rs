use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    pub r#type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
    #[serde(default)]
    pub disputed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_correct_transactions() {
        // has leading or trailing whitespace for some fields, missing or included optional last field
        // TODO: add more tests for less obvious edge cases
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
                amount: Some(1.0),
                disputed: false,
            },
            Transaction {
                r#type: TransactionType::Withdrawal,
                client: 2,
                tx: 2,
                amount: Some(2.1234),
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

        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(csv.as_bytes());
        let records = rdr
            .deserialize()
            .map(|result| {
                let record: Transaction = result.unwrap();
                record
            })
            .collect::<Vec<Transaction>>();
        assert_eq!(records, expected);
    }

    #[test]
    fn deserialize_invalid_type() {
        let csv = "type,client,tx,amount
invalid,1,1,1.0";

        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(csv.as_bytes());
        let result = rdr
            .deserialize::<Transaction>()
            .collect::<Result<Vec<Transaction>, csv::Error>>();
        println!("{:?}", result);
        assert!(result.is_err());
    }
}

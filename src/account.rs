use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Account {
    client: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
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

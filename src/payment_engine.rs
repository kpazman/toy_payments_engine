use std::collections::HashMap;
use thiserror::Error;

use crate::{Account, Transaction, TransactionType};

pub struct PaymentEngine {
    // account are stored in a hashmap, so it is faster to find by client id
    accounts: HashMap<u16, Account>,
}

#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Account {0} is locked")]
    AccountLocked(u16),
    #[error("Account {0} has insufficient funds for transaction {1}")]
    InsufficientFunds(u16, u32),
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Get account by client id, create on first access
    fn get_account(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert_with(|| {
            log::debug!("Adding account for client ID: {}", client);
            Account::new(client)
        })
    }

    /// Serialize all accounts to CSV string
    pub fn serialize_accounts(&self) -> anyhow::Result<String> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        for account in self.accounts.values() {
            writer.serialize(account)?;
        }
        let result = writer.into_inner()?;
        Ok(String::from_utf8(result)?)
    }

    pub fn process_transaction(&mut self, transaction: Transaction) -> Result<(), PaymentError> {
        log::debug!("Processing transaction: {:?}", transaction);

        // handle locked accounts early
        if self.get_account(transaction.client).is_locked() {
            return Err(PaymentError::AccountLocked(transaction.client));
        }

        match transaction.r#type {
            TransactionType::Deposit => self.deposit(transaction),
            TransactionType::Withdrawal => self.withdraw(transaction),
            TransactionType::Dispute => todo!(),
            TransactionType::Resolve => todo!(),
            TransactionType::Chargeback => todo!(),
        }
    }

    fn deposit(&mut self, transaction: Transaction) -> Result<(), PaymentError> {
        // transaction.amount is Some(f64) for TransactionType::Deposit, so unwrap is safe, TODO: enforce it better
        self.get_account(transaction.client)
            .deposit(transaction.amount.unwrap());

        Ok(())
    }

    fn withdraw(&mut self, transaction: Transaction) -> Result<(), PaymentError> {
        let account = self.get_account(transaction.client);

        // transaction.amount is Some(f64) for TransactionType::Withdrawal, so unwrap is safe, TODO: enforce it better
        if account.get_available() < transaction.amount.unwrap() {
            return Err(PaymentError::InsufficientFunds(
                transaction.client,
                transaction.tx,
            ));
        }

        account.withdraw(transaction.amount.unwrap());

        Ok(())
    }
}

impl Default for PaymentEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_locked_account() {
        let mut account = Account::new(1);
        account.lock();

        let mut payment_engine = PaymentEngine {
            accounts: HashMap::from([(1, account)]),
        };

        let transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
        };

        let res = payment_engine.process_transaction(transaction);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::AccountLocked(1));
    }

    #[test]
    fn process_deposit() {
        let mut payment_engine = PaymentEngine::new();

        let transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
        };

        payment_engine.process_transaction(transaction).unwrap();

        let expected_accounts = "client,available,held,total,locked
1,1.0,0.0,1.0,false
";

        let actual_accounts = payment_engine.serialize_accounts().unwrap();
        assert_eq!(actual_accounts, expected_accounts);
    }

    #[test]
    fn process_succesful_withdrawal() {
        let mut account = Account::new(1);
        account.deposit(1.0);

        let mut payment_engine = PaymentEngine {
            accounts: HashMap::from([(1, account)]),
        };

        let transaction = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(1.0),
        };

        payment_engine.process_transaction(transaction).unwrap();

        let expected_accounts = "client,available,held,total,locked
1,0.0,0.0,0.0,false
";

        let actual_accounts = payment_engine.serialize_accounts().unwrap();
        assert_eq!(actual_accounts, expected_accounts);
    }

    #[test]
    fn process_failed_withdrawal() {
        let mut payment_engine = PaymentEngine {
            accounts: HashMap::from([(1, Account::new(1))]),
        };

        let transaction = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(1.0),
        };

        let res = payment_engine.process_transaction(transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::InsufficientFunds(1, 1));
    }
}

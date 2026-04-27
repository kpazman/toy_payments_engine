use std::collections::HashMap;
use thiserror::Error;

use crate::{Account, Transaction, TransactionType};

// PaymentEngine stores accessed accounts and processed transactions as members in memory, that should be normally stored in a database
pub struct PaymentEngine {
    // account are stored in a hashmap, so it is faster to find by client id
    accounts: HashMap<u16, Account>,
    // transactions are stored in a hashmap, so it is faster to find by transaction id
    transactions: HashMap<u32, Transaction>,
}

#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Account {0} is locked")]
    AccountLocked(u16),
    #[error("Account {0} has insufficient funds for transaction {1}")]
    InsufficientFunds(u16, u32),
    #[error("Transaction {0} not found")]
    TransactionNotFound(u32),
    #[error("Transaction {0} has invalid type (no amount provided)")]
    InvalidTransctionType(u32),
    #[error("Transaction {0} is already under dispute")]
    TransactionUnderDispute(u32),
    #[error("Transaction {0} is not under dispute")]
    TransactionNotUnderDispute(u32),
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, transaction: Transaction) -> Result<(), PaymentError> {
        log::debug!("Processing transaction: {:?}", transaction);

        // handle locked accounts early
        if self.get_account(transaction.client).is_locked() {
            return Err(PaymentError::AccountLocked(transaction.client));
        }

        match transaction.r#type {
            TransactionType::Deposit => self.deposit(&transaction),
            TransactionType::Withdrawal => self.withdraw(&transaction),
            TransactionType::Dispute => self.dispute(&transaction),
            TransactionType::Resolve => self.resolve(&transaction),
            TransactionType::Chargeback => self.chargeback(&transaction),
        }?;

        self.store_transaction(transaction);

        Ok(())
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

    /// Get account by client id, create on first access
    fn get_account(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert_with(|| {
            log::debug!("Adding account for client ID: {}", client);
            Account::new(client)
        })
    }

    /// Get amount under dispute by transaction id
    fn get_disputed_amount(&self, transaction: &Transaction) -> Result<f64, PaymentError> {
        let disputed_transaction = self
            .transactions
            .get(&transaction.tx)
            .ok_or(PaymentError::TransactionNotFound(transaction.tx))?;

        if transaction.r#type == TransactionType::Dispute && disputed_transaction.disputed {
            return Err(PaymentError::TransactionUnderDispute(transaction.tx));
        }

        if (transaction.r#type == TransactionType::Resolve
            || transaction.r#type == TransactionType::Chargeback)
            && !disputed_transaction.disputed
        {
            return Err(PaymentError::TransactionNotUnderDispute(transaction.tx));
        }

        disputed_transaction
            .amount
            .ok_or(PaymentError::InvalidTransctionType(transaction.tx))
    }

    /// Store the transaction for Deposit/Withdrawal, update the disputed status for Dispute/Resolve/Chargeback
    fn store_transaction(&mut self, transaction: Transaction) {
        match transaction.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                self.transactions
                    .insert(transaction.tx, transaction.clone());
            }
            TransactionType::Dispute => {
                let mut updated_transaction =
                    self.transactions.get(&transaction.tx).unwrap().clone();
                updated_transaction.disputed = true;
                self.transactions
                    .insert(transaction.tx, updated_transaction);
            }
            TransactionType::Resolve | TransactionType::Chargeback => {
                let mut updated_transaction =
                    self.transactions.get(&transaction.tx).unwrap().clone();
                updated_transaction.disputed = false;
                self.transactions
                    .insert(transaction.tx, updated_transaction);
            }
        }
    }

    fn deposit(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
        // transaction.amount is Some(f64) for TransactionType::Deposit, so unwrap is safe, TODO: enforce it better
        self.get_account(transaction.client)
            .deposit(transaction.amount.unwrap());

        Ok(())
    }

    fn withdraw(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
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

    fn dispute(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
        let amount = self.get_disputed_amount(transaction)?;
        let account = self.get_account(transaction.client);
        account.dispute(amount);
        Ok(())
    }

    fn resolve(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
        let amount = self.get_disputed_amount(transaction)?;
        let account = self.get_account(transaction.client);
        account.resolve(amount);
        Ok(())
    }

    fn chargeback(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
        let amount = self.get_disputed_amount(transaction)?;
        let account = self.get_account(transaction.client);
        account.chargeback(amount);
        account.lock();
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
            transactions: HashMap::new(),
        };

        let transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
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
            disputed: false,
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
            transactions: HashMap::new(),
        };

        let transaction = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
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
            transactions: HashMap::new(),
        };

        let transaction = Transaction {
            r#type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let res = payment_engine.process_transaction(transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::InsufficientFunds(1, 1));
    }

    #[test]
    fn process_successful_dispute() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction)
            .unwrap();

        let expected_accounts = "client,available,held,total,locked
1,0.0,1.0,1.0,false
";

        let actual_accounts = payment_engine.serialize_accounts().unwrap();
        assert_eq!(actual_accounts, expected_accounts);
    }

    #[test]
    fn process_nonexistent_dispute() {
        let mut payment_engine = PaymentEngine::new();

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let res = payment_engine.process_transaction(dispute_transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::TransactionNotFound(1));
    }

    #[test]
    fn process_double_dispute() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction.clone())
            .unwrap();
        let res = payment_engine.process_transaction(dispute_transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::TransactionUnderDispute(1));
    }

    #[test]
    fn process_successful_resolve() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let resolve_transaction = Transaction {
            r#type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction)
            .unwrap();
        payment_engine
            .process_transaction(resolve_transaction)
            .unwrap();

        let expected_accounts = "client,available,held,total,locked
1,1.0,0.0,1.0,false
";

        let actual_accounts = payment_engine.serialize_accounts().unwrap();
        assert_eq!(actual_accounts, expected_accounts);
    }

    #[test]
    fn process_nonexistent_resolve() {
        let mut payment_engine = PaymentEngine::new();

        let resolve_transaction = Transaction {
            r#type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let res = payment_engine.process_transaction(resolve_transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::TransactionNotFound(1));
    }

    #[test]
    fn process_double_resolve() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let resolve_transaction = Transaction {
            r#type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction)
            .unwrap();
        payment_engine
            .process_transaction(resolve_transaction.clone())
            .unwrap();
        let res = payment_engine.process_transaction(resolve_transaction);

        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            PaymentError::TransactionNotUnderDispute(1)
        );
    }

    #[test]
    fn process_successful_chargeback() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let chargeback_transaction = Transaction {
            r#type: TransactionType::Chargeback,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction)
            .unwrap();
        payment_engine
            .process_transaction(chargeback_transaction)
            .unwrap();

        let expected_accounts = "client,available,held,total,locked
1,0.0,0.0,0.0,true
";

        let actual_accounts = payment_engine.serialize_accounts().unwrap();
        assert_eq!(actual_accounts, expected_accounts);
    }

    #[test]
    fn process_nonexistent_chargeback() {
        let mut payment_engine = PaymentEngine::new();

        let chargeback_transaction = Transaction {
            r#type: TransactionType::Chargeback,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let res = payment_engine.process_transaction(chargeback_transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::TransactionNotFound(1));
    }

    #[test]
    fn process_undisputed_chargeback() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let chargeback_transaction = Transaction {
            r#type: TransactionType::Chargeback,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        let res = payment_engine.process_transaction(chargeback_transaction);

        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            PaymentError::TransactionNotUnderDispute(1)
        );
    }

    #[test]
    fn process_double_chargeback() {
        let mut payment_engine = PaymentEngine::new();

        let depostit_transaction = Transaction {
            r#type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let dispute_transaction = Transaction {
            r#type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        let chargeback_transaction = Transaction {
            r#type: TransactionType::Chargeback,
            client: 1,
            tx: 1,
            amount: Some(1.0),
            disputed: false,
        };

        payment_engine
            .process_transaction(depostit_transaction)
            .unwrap();
        payment_engine
            .process_transaction(dispute_transaction)
            .unwrap();
        payment_engine
            .process_transaction(chargeback_transaction.clone())
            .unwrap();
        let res = payment_engine.process_transaction(chargeback_transaction);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), PaymentError::AccountLocked(1));
    }
}

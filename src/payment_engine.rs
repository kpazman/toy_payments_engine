use std::{collections::HashMap, io::Read, path::PathBuf};
use thiserror::Error;

use crate::{
    account::Account,
    transaction::{Transaction, TransactionStream, TransactionType},
};

// PaymentEngine stores accessed accounts and processed transactions as members in memory, that should be normally stored in a database
pub struct PaymentEngine {
    // account are stored in a hashmap, so it is faster to find by client id
    accounts: HashMap<u16, Account>,
    // transactions are stored in a hashmap, so it is faster to find by transaction id
    transactions: HashMap<u32, Transaction>,
}

/// Type representing errors in transaction processing logic
#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Account {0} is locked, referenced in transaction {1}")]
    AccountLocked(u16, Transaction),
    #[error("Transaction with ID {0} was already processed, referenced in transaction {1}")]
    TransactionIDNotUnique(u32, Transaction),
    #[error("Account {0} has insufficient funds for transaction {1}")]
    InsufficientFunds(u16, Transaction),
    #[error("Transaction ID {0} not found, referenced in transaction {1}")]
    TransactionNotFound(u32, Transaction),
    #[error("Transaction {0} does not belong to the account {1}, referenced in transaction {2}")]
    InconsistentDisputeRequest(u32, u16, Transaction),
    #[error("Transaction {0} is already under dispute, referenced in transaction {1}")]
    TransactionUnderDispute(u32, Transaction),
    #[error("Transaction {0} is not under dispute, referenced in transaction {1}")]
    TransactionNotUnderDispute(u32, Transaction),
    #[error("Transaction {0} has invalid type (no amount provided), referenced in transaction {1}")]
    InvalidTransctionType(u32, Transaction),
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Read transactions from CSV file and process them
    /// Expected format: type,client,tx,amount
    /// ```csv
    /// type, client, tx, amount
    /// deposit, 1, 1, 1.0
    /// dispute, 1, 1
    /// ...
    /// ```
    pub fn process_transactions_from_file(&mut self, file: &PathBuf) -> std::io::Result<()> {
        log::debug!("Reading transactions from {}", file.display());
        self.process_transaction_stream(TransactionStream::from_file(file)?);
        Ok(())
    }

    /// Process transactions in a [TransactionStream]. Errors are logged and processing continues.
    pub fn process_transaction_stream<R: Read>(&mut self, stream: TransactionStream<R>) {
        for result in stream {
            if let Ok(transaction) = result {
                if let Err(e) = self.process_transaction(&transaction) {
                    // log transaction processing error, then continue
                    log::warn!("{}", e);
                }
            } else {
                // log transaction parsing error, then continue
                log::warn!("{}", result.unwrap_err());
            }
        }
    }

    /// Process a single transaction
    pub fn process_transaction(&mut self, transaction: &Transaction) -> Result<(), PaymentError> {
        log::debug!("Processing transaction: {}", transaction);

        let account = self.get_account(transaction.client);
        log::debug!("Affected account before processing: {}", account);

        // handle locked accounts early
        if account.is_locked() {
            return Err(PaymentError::AccountLocked(
                transaction.client,
                transaction.clone(),
            ));
        }

        // check if transaction ID is unique for Deposit and Withdrawal transactions
        match transaction.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if self.transactions.contains_key(&transaction.tx) {
                    return Err(PaymentError::TransactionIDNotUnique(
                        transaction.tx,
                        transaction.clone(),
                    ));
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => (),
        };

        match transaction.r#type {
            TransactionType::Deposit => self.deposit(transaction),
            TransactionType::Withdrawal => self.withdraw(transaction),
            TransactionType::Dispute => self.dispute(transaction),
            TransactionType::Resolve => self.resolve(transaction),
            TransactionType::Chargeback => self.chargeback(transaction),
        }?;

        self.store_transaction(transaction);

        let account = self.get_account(transaction.client);
        log::debug!("Affected account after processing: {}", account);

        Ok(())
    }

    /// Serialize all accounts to CSV string
    /// Output format:
    /// ```csv
    /// client,available,held,total,locked
    /// 1,1.0,0.0,1.0,false
    /// ...
    /// ```
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

    /// Get amount under dispute by transaction id, verify that the referenced transaction belongs to the referenced account
    fn get_disputed_amount(&self, transaction: &Transaction) -> Result<f64, PaymentError> {
        let disputed_transaction =
            self.transactions
                .get(&transaction.tx)
                .ok_or(PaymentError::TransactionNotFound(
                    transaction.tx,
                    transaction.clone(),
                ))?;

        if transaction.client != disputed_transaction.client {
            return Err(PaymentError::InconsistentDisputeRequest(
                transaction.tx,
                transaction.client,
                transaction.clone(),
            ));
        }

        if transaction.r#type == TransactionType::Dispute && disputed_transaction.disputed {
            return Err(PaymentError::TransactionUnderDispute(
                transaction.tx,
                transaction.clone(),
            ));
        }

        if (transaction.r#type == TransactionType::Resolve
            || transaction.r#type == TransactionType::Chargeback)
            && !disputed_transaction.disputed
        {
            return Err(PaymentError::TransactionNotUnderDispute(
                transaction.tx,
                transaction.clone(),
            ));
        }

        // only Deposit and Withdrawal transactions are stored, so unwrap would safe, the or branch is unreachable
        disputed_transaction
            .amount
            .ok_or(PaymentError::InvalidTransctionType(
                transaction.tx,
                transaction.clone(),
            ))
    }

    /// Store the transaction for Deposit/Withdrawal, update the disputed status for Dispute/Resolve/Chargeback
    fn store_transaction(&mut self, transaction: &Transaction) {
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
                transaction.clone(),
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

        let res = payment_engine.process_transaction(&transaction);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            PaymentError::AccountLocked(1, transaction.clone())
        );
    }
}

use crate::{
    payment_engine::{PaymentEngine, PaymentError},
    transaction::{Transaction, TransactionType},
};
use rust_decimal::dec;

#[test]
fn process_deposit() {
    let mut payment_engine = PaymentEngine::new();

    let transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    payment_engine.process_transaction(&transaction).unwrap();

    let expected_accounts = "client,available,held,total,locked
1,1.0000,0.0000,1.0000,false
";

    let actual_accounts = payment_engine.serialize_accounts().unwrap();
    assert_eq!(actual_accounts, expected_accounts);
}

#[test]
fn process_duplicate_deposit() {
    let mut payment_engine = PaymentEngine::new();

    let transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    payment_engine.process_transaction(&transaction).unwrap();
    let res = payment_engine.process_transaction(&transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionIDNotUnique(1, transaction.clone())
    );
}

#[test]
fn process_successful_withdrawal() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let withdrawal_transaction =
        Transaction::new(TransactionType::Withdrawal, 1, 2, Some(dec!(1.0)), false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&withdrawal_transaction)
        .unwrap();

    let expected_accounts = "client,available,held,total,locked
1,0.0000,0.0000,0.0000,false
";

    let actual_accounts = payment_engine.serialize_accounts().unwrap();
    assert_eq!(actual_accounts, expected_accounts);
}

#[test]
fn process_failed_withdrawal() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();
    let withdrawal_transaction =
        Transaction::new(TransactionType::Withdrawal, 1, 2, Some(dec!(2.0)), false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&withdrawal_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::InsufficientFunds(1, withdrawal_transaction.to_record().unwrap())
    );
}

#[test]
fn process_duplicate_withdrawal() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let withdrawal_transaction =
        Transaction::new(TransactionType::Withdrawal, 1, 2, Some(dec!(1.0)), false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&withdrawal_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&withdrawal_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionIDNotUnique(2, withdrawal_transaction.clone())
    );
}

#[test]
fn process_successful_dispute() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();

    let expected_accounts = "client,available,held,total,locked
1,0.0000,1.0000,1.0000,false
";

    let actual_accounts = payment_engine.serialize_accounts().unwrap();
    assert_eq!(actual_accounts, expected_accounts);
}

#[test]
fn process_nonexistent_dispute() {
    let mut payment_engine = PaymentEngine::new();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let res = payment_engine.process_transaction(&dispute_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionNotFound(1, dispute_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_double_dispute() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&dispute_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionUnderDispute(1, dispute_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_inconsistent_dispute() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 2, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&dispute_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::InconsistentDisputeRequest(1, 2, dispute_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_successful_resolve() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let resolve_transaction =
        Transaction::new(TransactionType::Resolve, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&resolve_transaction)
        .unwrap();

    let expected_accounts = "client,available,held,total,locked
1,1.0000,0.0000,1.0000,false
";

    let actual_accounts = payment_engine.serialize_accounts().unwrap();
    assert_eq!(actual_accounts, expected_accounts);
}

#[test]
fn process_nonexistent_resolve() {
    let mut payment_engine = PaymentEngine::new();

    let resolve_transaction =
        Transaction::new(TransactionType::Resolve, 1, 1, None, false).unwrap();

    let res = payment_engine.process_transaction(&resolve_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionNotFound(1, resolve_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_double_resolve() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let resolve_transaction =
        Transaction::new(TransactionType::Resolve, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&resolve_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&resolve_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionNotUnderDispute(1, resolve_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_inconsistent_resolve() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let resolve_transaction =
        Transaction::new(TransactionType::Resolve, 2, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&resolve_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::InconsistentDisputeRequest(1, 2, resolve_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_successful_chargeback() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let chargeback_transaction =
        Transaction::new(TransactionType::Chargeback, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&chargeback_transaction)
        .unwrap();

    let expected_accounts = "client,available,held,total,locked
1,0.0000,0.0000,0.0000,true
";

    let actual_accounts = payment_engine.serialize_accounts().unwrap();
    assert_eq!(actual_accounts, expected_accounts);
}

#[test]
fn process_nonexistent_chargeback() {
    let mut payment_engine = PaymentEngine::new();

    let chargeback_transaction =
        Transaction::new(TransactionType::Chargeback, 1, 1, None, false).unwrap();

    let res = payment_engine.process_transaction(&chargeback_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionNotFound(1, chargeback_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_undisputed_chargeback() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let chargeback_transaction =
        Transaction::new(TransactionType::Chargeback, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&chargeback_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::TransactionNotUnderDispute(1, chargeback_transaction.to_modifier().unwrap())
    );
}

#[test]
fn process_double_chargeback() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let chargeback_transaction =
        Transaction::new(TransactionType::Chargeback, 1, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&chargeback_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&chargeback_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::AccountLocked(1, chargeback_transaction.clone())
    );
}

#[test]
fn process_inconsistent_chargeback() {
    let mut payment_engine = PaymentEngine::new();

    let deposit_transaction =
        Transaction::new(TransactionType::Deposit, 1, 1, Some(dec!(1.0)), false).unwrap();

    let dispute_transaction =
        Transaction::new(TransactionType::Dispute, 1, 1, None, false).unwrap();

    let chargeback_transaction =
        Transaction::new(TransactionType::Chargeback, 2, 1, None, false).unwrap();

    payment_engine
        .process_transaction(&deposit_transaction)
        .unwrap();
    payment_engine
        .process_transaction(&dispute_transaction)
        .unwrap();
    let res = payment_engine.process_transaction(&chargeback_transaction);

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        PaymentError::InconsistentDisputeRequest(
            1,
            2,
            chargeback_transaction.to_modifier().unwrap()
        )
    );
}

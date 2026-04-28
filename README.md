# Toy Payment Engine

A simple toy payments engine that reads a series of transactions from a CSV, updates client accounts, handles disputes and chargebacks, and then outputs the state of clients accounts as a CSV.

## Usage

The input file is the first and only argument to the binary. Output is written to std out.

```bash
cargo run -- transactions.csv > accounts.csv
```

The content of `accounts.csv` is expected to be the same as `accounts_example.csv`.

A more complex example input is included, that can trigger the various exceptions that are handled by the payment engine.
Run the following command to observe the logs:

```bash
RUST_LOG=debug cargo run -- transactions_demo.csv
```


## AI disclaimer

The project was developed in Cursor, tab completions might have been accepted then modified as needed, but no coding agents were used.

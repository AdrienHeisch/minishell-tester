# MAXITEST FOR MINISHELL

## Installation 

Clone this repository into a subdirectory in your minishell project and run
```
cargo build --release && mv target/release/maxitest .
```

## Help

Feature highlight: parallel execution, sandboxing, watch for recompilation, TUI
interface

Recommended usage: Run ./maxitest run -bpqw *.csv in a dedicated terminal. This will run your
tests every time you recompile your program, printing only the first few errors.

Tests are stored in csv files. Use a spreadsheet editor for convenience.

For any xxx.csv file, a xxx.ignore file can contain a list of test ids to ignore. One id per
line, use # to add comments.

Try the import-emtran subcommand to get a few hundred tests.

-> ./maxitest import-emtran && ./maxitext run emtran_mandatory.csv

## Licensing stuff

This projects includes an embedded binary from the [bubblewrap](https://github.com/containers/bubblewrap) project. Please reach out
if this is a problem.

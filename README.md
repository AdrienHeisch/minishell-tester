# MAXITEST FOR MINISHELL

## Installation 

With a rust toolchain installed, clone this repository into a subdirectory in your minishell project and run
```
cargo build --release && mv target/release/maxitest .
```

If you're using a workstation at 42, the rust compiler and system libraries might be too outdated.
Try to use the ```./run-container.sh``` script instead.

## Help

Feature highlight: parallel execution, sandboxing, watch for recompilation

Recommended usage: Run ./maxitest run -bpqw *.csv in a dedicated terminal. This will run your
tests every time you recompile your program, printing only the first few errors.

Don't forget to ```./maxitest run --help```

Tests are stored in csv files. Use a spreadsheet editor for convenience.

For any xxx.csv file, a xxx.ignore file can contain a list of test ids to ignore. One id per
line, use # to add comments.

Try the import-emtran subcommand to get a few hundred tests :
```
./maxitest import-emtran && ./maxitext run emtran_mandatory.csv
```

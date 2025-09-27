# MAXITEST FOR MINISHELL

## Installation 

### Cargo
```
cargo build --release && mv target/release/maxitest .
```

### Nix
```
nix build && mv result/bin/maxitest .
```

### Container
If you're using a workstation at 42, the rust compiler and system libraries might be too outdated.
Try to use the ```./run-container.sh``` script instead.

## Tests

This program contains two importers that will fetch about 1800 tests written by other students.
Please leave an issue if you are one of these students and want me to remove one of the importers.

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

# Running Eclair

You can use Eclair either in interactive REPL mode or to execute a file once.

## REPL mode

Run Eclair with no positional arguments to start the interactive REPL:

```bash
eclair
```

## Run a file

Pass a Solidity file as a positional argument to execute it once instead of starting the REPL:

```bash
eclair ./scripts/example.sol
```

The file is executed with the same initialization as the REPL (builtins, project loading, and init files).
Note that this interprets the file in using the Eclair REPL, so [does not behave like a normal Solidity file](./differences_with_solidity.md).

# Comparison with other REPLs

There are already several REPLs available for Solidity, such as [chisel](https://book.getfoundry.sh/reference/chisel/) or [solidity-shell](https://github.com/tintinweb/solidity-shell).
These REPLs provide a full Solidity environment by executing the code in the EVM, which makes them very useful tools to use a playground for Solidity.

Eclair has a different goal: it tries to make contract interactions as easy as possible while keeping the Solidity syntax.
To make this possible, it uses a very different approach: it does not execute the code in the EVM, but instead it interprets the code and generates the required transactions to interact with the contract on the fly.

As a byproduct of this approach, Eclair is able to provide features that would otherwise be hard or impossible to implement in a full EVM environment, such as having access to transaction receipts.
It also makes the execution speed much faster, which is nice in an interactive environment, as it does not need to compile and execute the code in the EVM.

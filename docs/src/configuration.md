# Configuration

Eclair can be used without any configuration but it is possible to configure it to make the experience smoother.

## RPC URL

The RPC url can be configured in several ways:

- Using the `ETH_RPC_URL` environment variable
- Using the `--rpc-url` option in the command line
- Using the `repl.rpc(RPC_URL)` function inside a session

## Etherscan API Key

Eclair requires an Etherscan API key to fetch contract ABIs and interact with the Etherscan API.
The API key can be set using the `ETHERSCAN_API_KEY` environment variable, which will be available for all chains.
For a per-chain configuration, the following environment variables can be used:

- Ethereum: `ETHERSCAN_API_KEY`
- Optimism: `OP_ETHERSCAN_API_KEY`
- Gnosis Chain: `GNOSISSCAN_API_KEY`
- Polygon: `POLYGON_API_KEY`
- Polygon zkEVM: `POLYGON_ZKEVM_API_KEY`
- Base: `BASESCAN_API_KEY`
- Arbitrum: `ARBISCAN_API_KEY`
- Sepolia: `SEPOLIA_ETHERSCAN_API_KEY`

## Initial setup

To allow to load common contracts and perform any other setup, Eclair can be configured to run code at startup.
By default, Eclair will look for a file named `.eclair_init.sol` in the current directory as well as in the
`$HOME/.foundry` directory.
Eclair will look for a function called `setUp` and load everything defined there in the current environment.
Note that any function defined will be loaded as normal functions and can be called from the REPL.

Here is an example setup:

```javascript
function setUp() {
    repl.loadAbi("ERC20", "~/.foundry/abis/erc20.json");

    if (!repl.connected) return;

    chainid = block.chainid;
    if (chainid == 1) {
        usdc = ERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
    } else if (chainid == 10) {
        usdc = ERC20(0x0b2c639c533813f4aa9d7837caf62653d097ff85);
    }
}
```

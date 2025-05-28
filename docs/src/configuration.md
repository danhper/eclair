# Configuration

Eclair can be used without any configuration but it is possible to configure it to make the experience smoother.
Eclair will also load [Foundry's configuration](https://book.getfoundry.sh/reference/config/overview) which can be used for some settings such as the RPC URL and the Etherscan API key.
Using the global configuration at `~/.foundry/foundry.toml` can be convenient for such settings.

A [sample Foundry configuration file](https://github.com/danhper/eclair/blob/main/config/foundry.toml) can be found in the [config](https://github.com/danhper/eclair/blob/main/config) directory.

## RPC URL

The RPC url can be configured in several ways:

- Using the `ETH_RPC_URL` environment variable
- Using the `--rpc-url` option in the command line
- Using the `vm.rpc(RPC_URL)` function inside a session

### Using Foundry configuration file

The [`rpc_endpoints` section](https://book.getfoundry.sh/reference/config/testing#rpc_endpoints) of `foundry.toml` to set aliases for your RPC URLs.
Here is a sample configuration:

```toml
# ~/.foundry/foundry.toml
[rpc_endpoints]
mainnet = "https://eth.llamarpc.com"
optimism = "https://mainnet.optimism.io"
```

This then allows to use `vm.rpc("mainnet")` or `vm.rpc("optimism")` to connect to the respective networks.

## Etherscan API Key

Eclair requires an Etherscan API key to fetch contract ABIs and interact with the Etherscan API V2.
The API key can be set using the `ETHERSCAN_API_KEY` environment variable.

### Using Foundry configuration file

The [`etherscan` section](https://book.getfoundry.sh/reference/config/etherscan) of `foundry.toml` can be used to set the API key.

Here is a sample configuration:

```toml
# ~/.foundry/foundry.toml
[etherscan]
mainnet = { key = "ETHERSCAN_API_KEY" }
# Chain not covered by Etherscan API V2
sei = { key = "SEITRACE_API_KEY", url = "https://seitrace.com/pacific-1/api", chain = 1329 }
```


## Initial setup

To allow to load common contracts and perform any other setup, Eclair can be configured to run code at startup.
By default, Eclair will look for a file named `.eclair_init.sol` in the current directory as well as in the
`$HOME/.foundry` directory.
Eclair will look for a function called `setUp` and load everything defined there in the current environment.
Note that any function defined will be loaded as normal functions and can be called from the REPL.

Here is an example setup:

```javascript
function setUp() {
    abi.load("ERC20", "~/.foundry/abis/erc20.json");

    if (!vm.connected) return;

    chainid = block.chainid;
    if (chainid == 1) {
        usdc = ERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
    } else if (chainid == 10) {
        usdc = ERC20(0x0b2c639c533813f4aa9d7837caf62653d097ff85);
    }
}
```

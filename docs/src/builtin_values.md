# Builtin values

## Globals

### `_`

The underscore `_` variable stores the result of the last expression evaluated.

```javascript
>> 1 + 1
2
>> _
2
```


### `keccak256(bytes data) -> bytes32`

Returns the keccak256 hash of the input bytes.

```javascript
>> keccak256("hello")
0x1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8
```

### `type(any value) -> Type`

Returns the type of the input value.

```javascript
>> type(1)
uint256
>> type(uint256)
type(uint256)
```

### `format(any value, uint8? decimals, uint8? precision) -> string`

Returns a human-readable (when possible) representation of the input value.
The `decimals` parameter is used to specify the decimals scaling factor for the value and the precision parameter is used to specify the number of decimal places to display. These are only relevant for numerical values.

```javascript
>> format(2.54321e18)
"2.54"
>> format(2.54321e18, 18, 3)
"2.543"
>> format(2.54321e7, 6)
"25.43"
>> format(bytes32("foo"))
"foo"
```

## `repl` functions

### `repl.vars -> null`

Displays a list of the variables defined in the current session.

### `repl.types -> null`

Displays a list of the types (excluding builtins).

### `repl.connected -> bool`

Returns `true` if the REPL is connected to a node.

### `repl.rpc() -> string`

Returns the URL of the RPC the REPL is connected to.

```javascript
>> repl.rpc()
"https://mainnet.optimism.io/"
```

### `repl.rpc(string url) -> null`

Sets the URL of the RPC to use.

```javascript
>> repl.rpc("https://mainnet.optimism.io/")
```

If the [RPC URL](./configuration.md#rpc-url) is set in the configuration file, the argument can be the name of the alias instead of the full URL:

```javascript
>> repl.rpc("optimism")
```

### `repl.fork() | repl.fork(string url) -> string`

This creates a fork using Anvil.
If a URL is provided, it will fork that network, otherwise it will use the current RPC (`repl.rpc()`).
This returns the endpoint of the Anvil instance.

```javascript
>> repl.fork()
"http://localhost:54383/"
```

### `repl.startPrank(address account) -> address`

Starts a prank on the given account.
This only works if connected to an RPC that supports `anvil_impersonateAccount` (which is the case after calling `repl.fork()`).

```javascript
>> repl.startPrank(0xCdaa941eB36344c54139CB9d6337Bd2154BBeEfA)
0xCdaa941eB36344c54139CB9d6337Bd2154BBeEfA
```

### `repl.stopPrank()`

Stops the current prank.

```javascript
>> repl.stopPrank()
```

### `repl.deal(address account, uint256 balance)`

Sets the ETH balance of `account` to `balance`.
This only works if connected to an RPC that supports `anvil_setBalance` (which is the case after calling `repl.fork()`).

```javascript
>> repl.deal(0xCdaa941eB36344c54139CB9d6337Bd2154BBeEfA, 1e18)
```

### `repl.block() -> uint256 | string`

Returns the current block in use for contract calls.

```javascript
>> repl.block()
"latest"
```

NOTE: this is different from `block.number` which returns the current block number of the chain.

### `repl.block(uint256 number) | repl.block(string tag) | repl.block(bytes32 hash)`

Sets the block to use for contract calls.
Can be a number, a tag (e.g. "latest" or "safe"), or a block hash.

```javascript
>> repl.block(123436578)
>> repl.block()
123436578
```


### `repl.exec(string command) -> uint256`

Executes a command in the shell, displays the output and returns the exit code.

```javascript
>> code = repl.exec("cat ./foundry.toml")
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
>> code
0
```

### `repl.loadAbi(string name, string path, string? key) -> null`

Loads an ABI from a file. The `name` parameter is used to reference the ABI in the REPL, the `path` parameter is the path to the file containing the ABI, and the `key` parameter is used to specify the key in the JSON file if it is nested.

```javascript
repl.loadAbi("ERC20", "./ERC20.json")
dai = ERC20(0x6B175474E89094C44Da98b954EedeAC495271d0F)

repl.loadAbi("ERC20", "./CompiledERC20.json", "abi")
```

### `repl.fetchAbi(string name, address implementationAddress) -> string`

Fetches the ABI of a contract from Etherscan using the Etherscan API key.
The `name` parameter is used to reference the ABI in the REPL, and the `implementationAddress` parameter is the address of the contract.
In the case of a proxy contract, the address of the implementation contract should be provided.
See [contract management](./contracts_management.md#fetching-abis-from-etherscan) for more information about proxy handling.
See [Etherscan API Key configuration](./configuration.md#etherscan-api-key) for more information on how to set the API key.

```javascript
>> dai = repl.fetchAbi("DAI", 0x6B175474E89094C44Da98b954EedeAC495271d0F)
```

### `repl.account -> address | null`

Returns the currently loaded account or null if none.

### `repl.loadPrivateKey(string? privateKey) -> address`

Sets the current account to the one corresponding to the provided private key and returns the loaded address.
If no private key is provided, the user will be prompted to enter one.

### `repl.loadKeystore(string name, string? password) -> address`

Loads the keystore located in `~/.foundry/keystore/<name>` and sets the current account to the one corresponding to the keystore.
If the password is not provided as the second argument, it will be prompted.

See [Using a keystore](./account_management.md#using-a-keystore) for more information.

### `repl.listLedgerWallets(uint256 count) -> address[]`

Returns a list of `count` wallets in the ledger.

### `repl.loadLedger(uint256 index) -> address`

Sets the current account to the one at the given ledger index and returns the loaded address.
The index should match the order of the wallets returned by `repl.listLedgerWallets`, starting at 0.
Only the ledger live derivation path is supported.

## `console` functions

### `console.log(any... value) -> null`

Logs the values to the console.

```javascript
>> console.log(1, "foo", 0x6B175474E89094C44Da98b954EedeAC495271d0F)
1
"foo"
0x6B175474E89094C44Da98b954EedeAC495271d0F
```

## `json` functions

### `json.stringify(any value) -> string`

Converts the value to a JSON string.

```javascript
>> json.stringify((1, ["a", "b"]))
"[1,["a","b"]]"
```

## `fs` functions

### `fs.write(string path, string value) -> void`

Writes the value to the file at the given path.

```javascript
>> fs.write("./file.txt", "hello")
```

## `abi` functions

### `abi.encode(any... args) -> bytes`

Encodes the arguments according to the ABI encoding rules.
Behaves like the regular Solidity `abi.encode` function.

```javascript
>> abi.encode(uint8(1), 0x789f8F7B547183Ab8E99A5e0E6D567E90e0EB03B)
0x0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000789f8f7b547183ab8e99a5e0e6d567e90e0eb03b
```

### `abi.encodePacked(any... args) -> bytes`

Concatenates all the arguments without padding.
Behaves like the regular Solidity `abi.encodePacked` function.

```javascript
>> abi.encodePacked(uint8(1), 0x789f8F7B547183Ab8E99A5e0E6D567E90e0EB03B)
0x01789f8f7b547183ab8e99a5e0e6d567e90e0eb03b
```

### `abi.decode(bytes data, (type...)) -> any`

Decodes the data according to the ABI encoding rules, given the types.
Behaves like the regular Solidity `abi.decode` function.

```javascript
>> abi.decode(0x0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000789f8f7b547183ab8e99a5e0e6d567e90e0eb03b, (uint8, address))
(1, 0x789f8F7B547183Ab8E99A5e0E6D567E90e0EB03B)
```

### `abi.decodeData(bytes data) -> any`

Decodes the data (either function calldata or error data) using any registered ABI.

```javascript
>> abi.decodeData(0xa9059cbb000000000000000000000000789f8f7b547183ab8e99a5e0e6d567e90e0eb03b0000000000000000000000000000000000000000000000000de0b6b3a7640000)
("transfer(address,uint256)", (0x789f8F7B547183Ab8E99A5e0E6D567E90e0EB03B, 1000000000000000000))
```

## `block` functions

### `block.number -> uint256`

Returns the current block number.

### `block.timestamp -> uint256`

Returns the current block timestamp.

### `block.basefee -> uint256`

Returns the current block base fee.

### `block.chainid -> uint256`

Returns the current chain ID.

## `events` functions

### `events.fetch{options}(address target) -> Log[] | events.fetch{options}(address[] targets) -> Log[]`

Fetches the events emitted by the contract(s) at the given address(es).
For more information, see [events](./interacting_with_contracts.md#events).

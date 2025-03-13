# Changelog

## Not released

### Features

- Refactor account management functions:
  - Move all account management functions to `accounts` namespace:
    - Rename `repl.current` to `accounts.current`
  - Allow to select already loaded accounts without loading them again
  - Add more management functions to `accounts` namespace:
    - `accounts.loaded` to list all loaded accounts
    - `accounts.select` to select a loaded account
    - `accounts.listKeystores` to list all keystores in `~/.foundry/keystore/`
    - `accounts.alias` to set an alias for an account

### Bug fixes

- Fix fixed bytes parsing/casting

### Other changes

- Add 0x prefix to JSON serialized bytes
- Rename `repl.loadAbi` to `abi.load` and `repl.fetchAbi` to `abi.fetch`
- Rename `repl.connected` to `vm.connected`

## v0.1.4 (2025-01-26)

### Features

- Add `repl.fork` to start run and use an Anvil instance as a fork of the current URL
- Add `repl.startPrank` / `repl.stopPrank` to start/stop impersonating an address
- Add `FUNC.traceCall` method to contract functions
- Add `abi.decodeData` to decode function calldata and errors from any known ABI
- Add `address.transfer` to send ETH to an address
- Add `json.stringify` to convert a value to a JSON string
- Add `fs.write` to write a string to a file
- Add `vm.skip` and `vm.mine` to skip time and mine blocks

### Other changes

- Encode `FixedBytes` as with zero padding on the left instead of the right
- Allow to load a private key from `bytes32`
- Move `repl.block`, `repl.startPrank`, `repl.stopPrank`, `repl.rpc`, `repl.fork`, and `repl.deal` to `vm` namespace

## v0.1.3 (2024-08-15)

### Features

- Decode logs returned by `Transaction.getReceipt()` when available in ABI
- Add support for array concatenation
- Add `array.filter` function
- Add `array.reduce` function
- Add support for bitwise operators
- [EXPERIMENTAL] Add support for anonymous functions
- [EXPERIMENTAL] Add support for fetching events
- Add support for negative indices in arrays and slices

### Bug fixes

- Fix `abi.decode` for nested types
- Fix completion not triggering right after `[`

### Other changes

- Drop `Receipt` type and use `NamedTuple` instead

## v0.1.2 (2024-08-04)

### Features

- Add `repl.block()`
- Allow to customize more variables through call options:
  - `block` - block number to use for the call
  - `from` - address to use as `msg.sender`
  - `gasLimit` - gas limit to use for the transaction
  - `maxFee` - maximum fee to pay for the transaction
  - `priorityFee` - priority fee to pay for the transaction
  - `gasPrice` - gas price to use for the (legacy) transaction
- Allow to select version of Eclair when installing using install script
- Add `repl.loadKeystore` to load keystore from a file created by `cast`
- Allow conversion between different fixed-size bytes types (e.g. bytes32 -> bytes4 or vice-versa)
- Allow slicing on bytes and strings

### Bug fixes

- Fix parsing of fix bytes with less than 32 bytes (e.g. bytes4)
- Fix display of functions that don't check argument types

## v0.1.1 (2024-07-30)

### Features

- Add support for bytes<->uint256 conversion

### Bug fixes

- Fix bug with `abi.encode` and `abi.decode` that caused the functions to fail/return incorrect values

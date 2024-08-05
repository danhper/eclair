# Changelog

## Not released

### Features

* Decode logs returned by `Transaction.getReceipt()` when available in ABI

### Other changes

* Drop `Receipt` type and use `NamedTuple` instead

## v0.1.2 (2024-08-04)

### Features

* Add `repl.block()`
* Allow to customize more variables through call options:
  * `block` - block number to use for the call
  * `from` - address to use as `msg.sender`
  * `gasLimit` - gas limit to use for the transaction
  * `maxFee` - maximum fee to pay for the transaction
  * `priorityFee` - priority fee to pay for the transaction
  * `gasPrice` - gas price to use for the (legacy) transaction
* Allow to select version of Eclair when installing using install script
* Add `repl.loadKeystore` to load keystore from a file created by `cast`
* Allow conversion between different fixed-size bytes types (e.g. bytes32 -> bytes4 or vice-versa)
* Allow slicing on bytes and strings

### Bug fixes

* Fix parsing of fix bytes with less than 32 bytes (e.g. bytes4)
* Fix display of functions that don't check argument types

## v0.1.1 (2024-07-30)

### Features

* Add support for bytes<->uint256 conversion

### Bug fixes

* Fix bug with `abi.encode` and `abi.decode` that caused the functions to fail/return incorrect values

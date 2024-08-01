# Changelog

## Not released

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

## 0.1.1 (2024-07-30)

### Features

* Add support for bytes<->uint256 conversion

### Bug fixes

* Fix bug with `abi.encode` and `abi.decode` that caused the functions to fail/return incorrect values

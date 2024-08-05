# Builtin methods

## Global methods

These methods are available on all types.

### `any.format() -> string`

Returns a human-readable (when possible) representation of the value.
See the [format function](./builtin_functions.md#formatany-value-uint8-decimals-uint8-precision---string) for more details.


## `string` methods

### `string.length -> uint256`

Returns the length of the string.

```javascript
>> "foo".length
3
```

### `string.concat(string other) -> string`

Concatenates two strings.

```javascript
>> "foo".concat("bar")
"foobar"
```

## `bytes` methods

### `bytes.length -> uint256`

Returns the length of the bytes.

### `bytes.concat(bytes other) -> bytes`

Concatenates two byte arrays.


## `array` methods

### `array.length -> uint256`

Returns the length of the array.

### `array.map(function f) -> array`

Applies the function `f` to each element of the array and returns a new array with the results.

```javascript
>> function double(x) { return x * 2; }
>> [1, 2, 3].map(double)
[2, 4, 6]
```

### `array.concat(array other) -> array`

Concatenates two arrays.

```javascript
>> [1, 2].concat([3, 4])
[1, 2, 3, 4]
```

## `tuple` methods

### `tuple[uint256 index] -> any`

Returns the element at the given index.

### `tuple.length -> uint256`

Returns the length of the tuple.

### `tuple.map(function f) -> array`

Applies the function `f` to each element of the tuple and returns a new tuple with the results.

## `address` methods

### `address.balance -> uint256`

Returns the balance of the address.

## `num` (`uint*` and `int*`) methods

### `num.mul(num other) -> num` | `num.mul(num other, uint8 decimals) -> num`

Multiplies two scaled numbers. By default, the number are assumed to have 18 decimals.
The second argument can be used to specify the number of decimals.

```javascript
>> 2e18.mul(3e18)
6000000000000000000
```

### `num.div(num other) -> num` | `num.div(num other, uint8 decimals) -> num`

Divides two scaled numbers using similar logic to `mul`.

## `Transaction` methods

### `Transaction.getReceipt() -> Receipt` | `Transaction.getReceipt(uint256 timeout) -> Receipt`

Returns the receipt of the transaction.
The function will wait for the transaction for up to `timeout` seconds, or 30 seconds by default.

```javascript
>> tx = Transaction(0xfb89e2333b81f2751eedaf2aeffb787917d42ea6ea7c5afd4d45371f3f1b8079)
>> tx.getReceipt()
Receipt { txHash: 0xfb89e2333b81f2751eedaf2aeffb787917d42ea6ea7c5afd4d45371f3f1b8079, blockHash: 0xd82cbdd9aba2827815d8db2e0665b1f54e6decc4f59042e53344f6562301e55b, blockNumber: 18735365, status: true, gasUsed: 54017, gasPrice: 71885095452, logs: [Log { address: 0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A, topics: [0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef, 0x00000000000000000000000035641673a0ce64f644ad2f395be19668a06a5616, 0x0000000000000000000000009748a9de5a2d81e96c2070f7f0d1d128bbb4d3c4], data: 0x00000000000000000000000000000000000000000000007b1638669932a6793d, args: Transfer { from: 0x35641673A0Ce64F644Ad2f395be19668A06A5616, to: 0x9748a9dE5A2D81e96C2070f7F0D1d128BbB4d3c4, value: 2270550663541970860349 } }] }
```

The receipt is a named tuple with the following fields:

- `txHash` (`bytes32`): the hash of the transaction.
- `blockHash` (`bytes32`): the hash of the block containing the transaction.
- `blockNumber` (`uint256`): the number of the block containing the transaction.
- `status` (`bool`): the status of the transaction.
- `gasUsed` (`uint256`): the amount of gas used by the transaction.
- `gasPrice` (`uint256`): the gas price of the transaction.
- `logs` (`NamedTuple[]`): the logs of the transaction.

Logs is an array of named tuples with the following fields:

- `address` (`address`): the address of the contract that emitted the log.
- `topics` (`bytes32[]`): the topics of the log.
- `data` (`bytes`): the data of the log.
- `args` (`NamedTuple`): the decoded arguments of the log, if the event is known (present in one of the loaded ABIs).


## `Contract` static methods

These methods are available on the contracts themselves, not on their instances.

### `Contract.decode(bytes data) -> (string, tuple)`

Decodes the ABI-encoded data into a tuple.
The first element of the tuple is the function signature, and the second element is the decoded arguments.

```javascript
>> ERC20.decode(0xa9059cbb000000000000000000000000789f8f7b547183ab8e99a5e0e6d567e90e0eb03b0000000000000000000000000000000000000000000000056bc75e2d63100000)
("transfer(address,uint256)", (0x789f8F7B547183Ab8E99A5e0E6D567E90e0EB03B, 100000000000000000000))
```

## `num` (`uint*` and `int*`) static methods

### `type(num).max -> num`

Returns the maximum value of the type.

```javascript
>> type(uint8).max
255
```

### `type(num).min -> num`

Returns the minimum value of the type.

```javascript
>> type(int8).min
-128
```

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
>> tx = Transaction(0xad8a96e212d8d95187bdbb06b44f63ab1a0718a4b4d9086cbd229b6bffc43089)
>> tx.getReceipt()
TransactionReceipt { tx_hash: 0xad8a96e212d8d95187bdbb06b44f63ab1a0718a4b4d9086cbd229b6bffc43089, block_hash: 0xec8704cc21a047e95287adad403138739b7b79fae7fd15aa14f1d315aae1db1f, block_number: 20387117, status: true, gas_used: 34742, gas_price: 3552818949 }
```

## `Receipt` methods

### `Receipt.status -> bool`

Returns the status of the transaction.

### `Receipt.tx_hash -> bytes32`

Returns the hash of the transaction.

### `Receipt.gas_used -> uint256`

Returns the amount of gas used by the transaction.

### `Receipt.effective_gas_price -> uint256`

Returns the gas price of the transaction.

### `Receipt.block_number -> uint256`

Returns the block number of the transaction.

### `Receipt.block_hash -> uint256`

Returns the block hash of the transaction.

### `Receipt.logs -> array`

Returns the logs of the transaction.

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

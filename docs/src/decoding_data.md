# Decoding Data

Eclair provides several functions for decoding ABI-encoded data, which can be useful when verifying transactions.

## Basic ABI Decoding

The `abi.decode` function works similarly to Solidity's `abi.decode`, allowing to decode ABI-encoded data with known types:

```javascript
>> encoded = abi.encode("Hello", 123)
>> (str, num) = abi.decode(encoded, (string, uint256))
>> str
"Hello"
>> num
123
```

## Decoding Function Call Data

For decoding function calls or error data from contracts, you can use `abi.decodeData`.
This function requires that the relevant contract ABI has been loaded (see [Contracts management](./contracts_management.md)):

```javascript
>> dai = ERC20(0x6b175474e89094c44da98b954eedeac495271d0f)
>> data = dai.transfer.encode(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045, 1e18)
>> abi.decodeData(data)
("transfer(address,uint256)", (0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045, 100000000000000000000))
```

Note that the output is a tuple and can be manipulated as such:

```javascript
>> (func, args) = abi.decodeData(data)
>> func
"transfer(address,uint256)"
>> args
(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045, 1000000000000000000)
>> args[1].format()
"1.00"
```

## Decoding Safe MultiSend Transactions

For decoding Gnosis Safe multiSend transactions, use `abi.decodeMultisend`:

```javascript
>> multisendData = 0x008a5eb9a5b726583a213c7e4de2403d2dfd42c8a600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044e2a4853ae060499125866d3940796528a5be3e30632cf5c956aae07e9b72d89c96e053f100000000000000000000000000000000000000000000000006f05b59d3b20000008a5eb9a5b726583a213c7e4de2403d2dfd42c8a600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044e2a4853a2a55eed44296e96ac21384858860ec77b2c3e06f2d82cbe24bc29993e5a520110000000000000000000000000000000000000000000000000de0b6b3a7640000
>> abi.decodeMultisend(multisendData)
[MultisendTransaction {
    operation: 0,
    to: 0x8A5eB9A5B726583a213c7e4de2403d2DfD42C8a6,
    value: 0,
    data: 0xe2a4853ae060499125866d3940796528a5be3e30632cf5c956aae07e9b72d89c96e053f100000000000000000000000000000000000000000000000006f05b59d3b20000
}, MultisendTransaction {
    operation: 0,
    to: 0x8A5eB9A5B726583a213c7e4de2403d2DfD42C8a6,
    value: 0,
    data: 0xe2a4853a2a55eed44296e96ac21384858860ec77b2c3e06f2d82cbe24bc29993e5a520110000000000000000000000000000000000000000000000000de0b6b3a7640000
}]
```

Each decoded transaction contains:
- `operation`: 0 for regular call, 1 for delegatecall
- `to`: Target address
- `value`: ETH value in wei
- `data`: Call data for the transaction

This can be combined with `abi.decodeData` to decode the data of each transaction. For example:

```javascript
>> abi.decodeMultisend(multisendData).map((d) >> (d, abi.decodeData(d.data)))
[(MultisendTransaction {
    operation: 0,
    to: 0x8A5eB9A5B726583a213c7e4de2403d2DfD42C8a6,
    value: 0,
    data: 0xe2a4853ae060499125866d3940796528a5be3e30632cf5c956aae07e9b72d89c96e053f100000000000000000000000000000000000000000000000006f05b59d3b20000
}, ("setUint(bytes32,uint256)", (0xe060499125866d3940796528a5be3e30632cf5c956aae07e9b72d89c96e053f1, 500000000000000000))
), (MultisendTransaction {
    operation: 0,
    to: 0x8A5eB9A5B726583a213c7e4de2403d2DfD42C8a6,
    value: 0,
    data: 0xe2a4853a2a55eed44296e96ac21384858860ec77b2c3e06f2d82cbe24bc29993e5a520110000000000000000000000000000000000000000000000000de0b6b3a7640000
}, ("setUint(bytes32,uint256)", (0x2a55eed44296e96ac21384858860ec77b2c3e06f2d82cbe24bc29993e5a52011, 1000000000000000000))
)]
```
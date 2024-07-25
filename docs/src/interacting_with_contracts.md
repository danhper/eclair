# Interacting with contracts

Eclair provides a simple way to interact with smart contracts using familiar Solidity syntax.

## Calling contracts and sending transactions

For any contract loaded, all the functions of the contract defined in its ABI are available as methods on the contract object.

By default, calling a view function will call it and return the result, while calling a non-view function will send a transaction and returns the transaction hash.
Sending a transaction requires an [account to be loaded](./account_management.md).

The behavior can be changed by using one of the following method on the returned function object:

* `call`: Call the function and return the result
* `send`: Sends a transaction to the function and return the result
* `encode`: ABI-encodes the function call

```javascript
>> dai = repl.fetchAbi("DAI", 0x6B175474E89094C44Da98b954EedeAC495271d0F)
>> dai.balanceOf(0x4DEDf26112B3Ec8eC46e7E31EA5e123490B05B8B)
49984400000000000000000000
>> repl.loadPrivateKey()
>> dai.balanceOf.send(0x4DEDf26112B3Ec8eC46e7E31EA5e123490B05B8B)
Transaction(0x6a2f1b956769d06257475d18ceeec9ee9487d91c97d36346a3cc84d568e36e5c)
>> dai.balanceOf.encode(0x4DEDf26112B3Ec8eC46e7E31EA5e123490B05B8B)
0x70a082310000000000000000000000004dedf26112b3ec8ec46e7e31ea5e123490b05b8b
>> dai.transfer(0x4DEDf26112B3Ec8eC46e7E31EA5e123490B05B8B, 1e18)
Transaction(0xf3e85039345ff864bb216b10e84c7d009e99ec55b370dae22706b0d48ea41583)
```

## Transaction receipts

After sending a transaction, you can get the transaction receipt using the `Transaction.getReceipt` method.

```javascript
>> tx = dai.approve(0x83F20F44975D03b1b09e64809B757c47f942BEeA, 1e18)
>> tx.getReceipt()
TransactionReceipt { tx_hash: 0x248ad948d1e4eefc6ccb271cac2001ebbdb2346beddc7656b1f9518f216c8b02, block_hash: 0x688517fe5e540b4e3953ed3ba84cc4d70903ddffb981a66c51ca49ca13c90bb1, block_number: 20380613, status: true, gas_used: 46146, gas_price: 4547819249 }
>> _.logs
[Log { address: 0x6B175474E89094C44Da98b954EedeAC495271d0F, topics: [0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925, 0x000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266, 0x00000000000000000000000083f20f44975d03b1b09e64809b757c47f942beea], data: 0x0000000000000000000000000000000000000000000000000de0b6b3a7640000 }]
```

Note: Event decoding is not implemented yet

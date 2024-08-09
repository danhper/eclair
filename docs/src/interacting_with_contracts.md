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

### Transaction options

There are different options available when calling and sending transactions to contracts.
The options can be passed using the `{key: value}` Solidity syntax, for example:

```javascript
>> tx = weth.deposit{value: 1e18}()
```

The following options are currently supported:

* `value`: sets the `msg.value` of the transaction
* `block`: sets the block number to execute the call on (only works for calls, not for sending transactions)
* `from`: sets the `from` for the call (only works for calls, not for sending transactions)
* `gasLimit`: sets the gas limit to use for the transaction
* `maxFee`: sets the maximum fee to pay for the transaction
* `priorityFee`: sets the priority fee to pay for the transaction
* `gasPrice`: sets gas price to use for the (legacy) transaction

## Transaction receipts

After sending a transaction, you can get the transaction receipt using the `Transaction.getReceipt` method.

```javascript
>> tx = dai.approve(0x83F20F44975D03b1b09e64809B757c47f942BEeA, 1e18)
>> tx.getReceipt()
TransactionReceipt { tx_hash: 0x248ad948d1e4eefc6ccb271cac2001ebbdb2346beddc7656b1f9518f216c8b02, block_hash: 0x688517fe5e540b4e3953ed3ba84cc4d70903ddffb981a66c51ca49ca13c90bb1, block_number: 20380613, status: true, gas_used: 46146, gas_price: 4547819249 }
>> _.logs
[Log { address: 0x6B175474E89094C44Da98b954EedeAC495271d0F, topics: [0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925, 0x000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266, 0x00000000000000000000000083f20f44975d03b1b09e64809b757c47f942beea], data: 0x0000000000000000000000000000000000000000000000000de0b6b3a7640000 }]
```

If the ABI of the contract emitting the log is loaded, the logs will automatically be decoded and the decoded arguments will be available in the `args` property of each log.

## Events

Eclair provides a way to fetch events emitted by a contract using the `events.fetch` method.

```javascript
>> events.fetch{fromBlock: 20490506, toBlock: 20490512}(0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A)[0]
Log { address: 0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A, topics: [0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef, 0x000000000000000000000000ba12222222228d8ba445958a75a0704d566bf2c8, 0x000000000000000000000000f081470f5c6fbccf48cc4e5b82dd926409dcdd67], data: 0x00000000000000000000000000000000000000000000000e8bd6d724bc4c7886, args: Transfer { from: 0xBA12222222228d8Ba445958a75a0704d566BF2C8, to: 0xf081470f5C6FBCCF48cC4e5B82Dd926409DcdD67, value: 268330894800999708806 } }
```

The `events.fetch` accepts either a single address or a list of addresses as the first argument, as well as some options
to filter the logs returned.
It returns a list of logs that match the given criteria, and automatically decodes each log if the ABI is loaded.

### Options

The `events.fetch` method accepts the following options:

* `fromBlock`: the block number to start fetching events from
* `toBlock`: the block number to stop fetching events at
* `topic0`: topic0 of the event
* `topic1`: topic1 of the event
* `topic2`: topic2 of the event
* `topic3`: topic3 of the event

By default, it will try to fetch from the first ever block to the latest block.
In many cases, the RPC provider will reject the request because too much data would be returned, in which case
options above will need to be added to restrict the size of the response.

To only get one type of event, e.g. `Transfer`, you can filter using `topic0` and the selector of the desired event.

```javascript
>> events.fetch{fromBlock: 20490506, toBlock: 20490512, topic0: ERC20.Approval.selector}(0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A)[0]
Log { address: 0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A, topics: [0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925, 0x0000000000000000000000008149dc18d39fdba137e43c871e7801e7cf566d41, 0x000000000000000000000000ea50f402653c41cadbafd1f788341db7b7f37816], data: 0x000000000000000000000000000000000000000000000025f273933db5700000, args: Approval { owner: 0x8149DC18D39FDBa137E43C871e7801E7CF566D41, spender: 0xeA50f402653c41cAdbaFD1f788341dB7B7F37816, value: 700000000000000000000 } }
```

# Eclair: a Solidity Interpreter

Eclair is a Solidity interpreter designed to provide a fast and intuitive [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop)
to interact with EVM smart contracts using Solidity.

Eclair is built on top of the [Solang](https://solang.readthedocs.io/en/v0.3.3/) for parsing and [Alloy](https://alloy.rs/) for most
Ethereum related features.

Eclair is designed to integrate well with the rest of the ecosystem, and can be used seamlessly with [Foundry](https://book.getfoundry.sh/), [Hardhat](https://hardhat.org/) and [Brownie](https://eth-brownie.readthedocs.io/en/stable/) on any EVM-compatible chain.


Although Eclair uses Solidity as a base for its syntax, the semantics are not designed to be exactly the same as those provided by the Solidity compiler.
In most cases, ease-of-use and simplicity are prioritized over compatibility with Solidity.

Here is a short example of an interaction using Eclair to interact with smart contracts deployed on [Optimism](https://optimism.io/) using a [Ledger](https://www.ledger.com/).

```python
>> repl.rpc("https://mainnet.optimism.io")
>> repl.fetchAbi("ERC20", 0xded3b9a8dbedc2f9cb725b55d0e686a81e6d06dc)
ERC20(0xdEd3b9a8DBeDC2F9CB725B55d0E686A81E6d06dC)
>> usdc = ERC20(0x0b2c639c533813f4aa9d7837caf62653d097ff85)
>> repl.loadLedger(5)
0x2Ed58a93c5Daf1f7D8a8b2eF3E9024CB6BFa9a77
>> usdc.balanceOf(repl.account).format(usdc.decimals())
"5.00"
>> swapper = repl.fetchAbi("Swapper", 0x956f9d69Bae4dACad99fF5118b3BEDe0EED2abA2)
>> usdc.approve(swapper, 2e6)
Transaction(0xed2cfee9d712fcaeb0bf42f98e45d09d9b3626a0ee93dfc730a3fb7a0cda8ff0)
>> targetAsset = repl.fetchAbi("LToken", 0xc013551a4c84bbcec4f75dbb8a45a444e2e9bbe7)
>> amountIn = 2e6
>> minOut = 2e18.div(targetAsset.exchangeRate()).mul(0.95e18)
>> minOut.format(targetAsset.decimals())
"4.53"
>> swapper.mint
Swapper(0x956f9d69Bae4dACad99fF5118b3BEDe0EED2abA2).mint(address zapAssetAddress_,
  address leveragedTokenAddress_,uint256 zapAssetAmountIn_,uint256 minLeveragedTokenAmountOut_)
>> tx = swapper.mint(usdc, targetAsset, amountIn, minOut)
>> receipt = tx.getReceipt()
>> receipt.tx_hash
0xbdbaddb66c696afa584ef93d0d874fcba090e344aa104f199ecb682717009691
>> receipt.gas_used
1803959
```

Note that this requires [Optimism Etherscan API key to be set](./configuration.md).

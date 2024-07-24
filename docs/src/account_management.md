# Account management

Loading an account is needed to send transactions to smart contract.
Eclair currently supports loading an account using a private key or a Ledger device.

## Using a private key

Accounts can be loaded from a private key, using the `repl.loadPrivateKey` function:

```javascript
>> repl.loadPrivateKey() // usage with no argument
Enter private key: 
0xec1a77322592C180Ca626C2f9DDe3976E5712011
>> repl.loadPrivateKey("4546df48889621143395cf30567352fab50ed9c48149836e726550f1361e43df") // passing the private key as an argument
0xec1a77322592C180Ca626C2f9DDe3976E5712011
>> repl.account
0xec1a77322592C180Ca626C2f9DDe3976E5712011
```

## Using a ledger

Accounts can be read from a Ledger device, using the `repl.loadLedger` function.
The `repl.listLedgerWallets()` function can be used to list the available wallets on the Ledger device.
The index of the wallet to load should be passed as an argument to the `repl.loadLedger` function.
Note that only the ledger live derivation is supported for now.

```javascript
>> repl.listLedgerWallets()
[0x4d09e5617C38F8A9884d79464B7BE1b12353eD05, 0x669F44bB2DFb534707E6FAE940d7558ab0FE254D, 0x5Ac61EbcEbf7De5D19a807752f13Cd7a9Af4Ffc4, 0xb3eBAf4686741C8a7A2Adf7738A1a84a883127c2, 0xC033068376264C0a5971b706894d3fc0eB93A2dD]
>>> repl.loadLedger(1)
0x669F44bB2DFb534707E6FAE940d7558ab0FE254D
>> repl.account
0x669F44bB2DFb534707E6FAE940d7558ab0FE254D
```

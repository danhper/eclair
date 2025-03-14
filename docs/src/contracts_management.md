# Contracts management

Eclair provides different ways to load contracts (or rather contract ABIs) to be able to interact with them.

## Loading from existing project

If `eclair` is ran in a directory using Foundry, Brownie or Hardhat, all the compiled contracts in the project will be loaded automatically.
No additional setup is needed. A list of all loaded contracts can be viewed using `repl.types`.
One caveat is that Eclair does not currently supports multiple contracts with the same name, so last occurrence will overwrite the previous one.

## Loading from an ABI file

Contracts can be loaded from a JSON ABI file using the `abi.load` function.
The first argument is the name of the contract, which will be defined in the environment.
The second argument is the path to the JSON file containing the ABI.

```javascript
abi.load("ERC20", "path/to/abi.json")
```

If the ABI is nested in the JSON file (e.g. under the `abi` key), the key can be specified as a third argument.

```javascript
abi.load("ERC20", "path/to/abi.json", "abi")
```

## Fetching ABIs from Etherscan

Contracts can be loaded from Etherscan using the `abi.fetch` function.
Note that this requires an [Etherscan API key to be set](./configuration.md).
The first argument is, as for `abi.load` the name of the contract, and the second argument is the address of the contract.

```javascript
dai = abi.fetch("DAI", 0x6B175474E89094C44Da98b954EedeAC495271d0F)
>> dai
DAI(0x6B175474E89094C44Da98b954EedeAC495271d0F)
```

For contracts that use a proxy pattern, the process can be split into two steps.

```javascript
abi.fetch("USDC", 0x43506849D7C04F9138D1A2050bbF3A0c054402dd); // implementation address
usdc = USDC(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48); // proxy address
```

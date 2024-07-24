# Differences with Solidity

Eclair is based on the Solidity syntax but behaves differently in many aspects.
It is worth nothing that at this point, Eclair only implements a subset of the Solidity language.

This section highlights some of the differences between Eclair and Solidity.

## Static vs dynamic typing

Solidity is statically typed, meaning that the type of each variable must be known at compile time and the compiler will check that the types are used correctly.
On the other hand, Eclair is not dynamically typed and variables are only checked at runtime. Furthermore, variables do not need to be declared explicitly.

```javascript
>> a = 1; // a is an integer
>> a + 2
3
>> a + "bar"
Error: cannot add uint256 and string
>> a = "foo"; // a is now a string
>> a + "bar"
"foobar"
```

## Scopes

Eclair has a different scoping mechanism compared to Solidity.
In particular, Eclair only creates a new scope for function, not for blocks.
This means that the following code will work in Eclair but not in Solidity:

```javascript
if (true) {
    a = 1;
} else {
    a = 2;
}
console.log(a);
```

However, the following code will not work:

```javascript
>> function foo() { a = 1; }
>> console.log(a)
Error: a is not defined
```

## First-class functions and types

Internally, almost everything in Eclair returns a value.
This means that functions and types can be passed around as arguments to other functions.
For example, the following is valid in Eclair:

```javascript
>> usdc = ERC20(0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48)
>> usdcBalance = usdc.balanceOf
>> usdcBalance(0x47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503)
542999999840000
>> tu16 = type(uint16)
>> tu16.max
65535
```

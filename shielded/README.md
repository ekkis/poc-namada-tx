# Building a Shielded Transaction

In this document we'll work to enhance the code built in the [previous tutorial](../simple/README.md) to support *shielded* transactions.  A shielded transaction is one that gets submitted to the MASP (an implementation of the Sapling Circuit designed to handle multiple types of assets), which means there is zero-knowledge left about the transaction

## Basic Setup

As before, we'll start by creating two keypairs, but this time shielded and called *customer* and *merchant*:

```bash
nm-wallet customer --shielded
nm-wallet merchant --shielded
```

A shielded keypair can declare multiple payment addresses but here we'll create only one for each of these players: *savings* for the customer and *sales*, which will go to the merchant's Accounts Receivable department

```bash
# encapsulate

nm-payaddr() {
    key=$1; shift
    namadaw gen-payment-addr --key=$key --alias=$*
}

# generate address

nm-payaddr customer savings
nm-payaddr merchant sales
```

The customer is incentivised to keep his funds in a shielded address because the MASP compensates him for it, in the same way banks used to compensate customers for keeping a balance

So let's give the customer some money to play with by sending token from the wallet in the validator, to the shielded address we created:

```bash
nm-fund $(nm-addr savings)
```

The above is referred to as a *shielding transaction* as it conceals the funds.  With this balance our code can now do a *shielded transaction* in paying the merchant

## Developer Setup

Let's create a new project (I've chosen to call it _shielded_ but you can call it what you like) by copying from the code for the previous tutorial"

```bash
cp -r simple shielded && cd shielded
```

...and let's fix the environment file to point to our new locations:

```bash
sed -ie 's/SOURCE.*/SOURCE=customer/' .env
sed -ie 's/TARGET.*/TARGET=sales/' .env
```

## Shielding the transaction

To handle shielded addresses we need to create a new function:

```rust
// top of file

use namada_sdk::types::masp::PaymentAddress;

// just above main()

fn get_shielded_addr(w: &Wallet<FsWalletUtils>, val: &String) -> PaymentAddress {
	let s = w.find_payment_addr(val).map(|addr| addr.to_string()).unwrap();
	PaymentAddress::from_str(&*s)
}
```

...and we then get the shielded address for the TARGET:

```rust
let target = get_shielded_addr(&wallet, &config.target);
```

also, we can no longer get the secret key for the sender in the way we used to, since shielded keypairs may have multiple spending keys and the object type is different.  Instead we must fetch it like this:

```rust
let sk = wallet.find_spending_key(&config.source, None)
		.expect("Unable to find key");
```

...and now we can create the transfer builder, but notice that the source takes the secret key directly, no longer an address:

```rust
let mut transfer_tx_builder = sdk.new_transfer(	
    TransferSource::ExtendedSpendingKey(sk),
    TransferTarget::PaymentAddress(target),
    token.clone(),
    InputAmount::Unvalidated(amt),
);
```

[« PREV](../simple/README.md) | [NEXT »](../IBC/README.md)
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

The above is referred to as a *shielding transaction* as it conceals the funds by transferring them from a *transparent* address to a secret one.  With this balance our code can now do a *shielded transaction* in paying the merchant

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

To speed things up (because shielded transactions take more work), we'll enhance our solution by adding the `multicore` feature to the SDK.  Make sure the `[dependencies]` entry looks like this:

```toml
namada_sdk = { git = "https://github.com/anoma/namada", tag = "v0.31.0", default-features = false, features = ["tendermint-rpc", "std", "async-client", "async-send", "download-params", "rand", "multicore"] }
```

## Shielding the transaction

Shielded transactions use *payment addresses*, so we need to create a new function to retrieve these:

```rust
// top of file

use namada_sdk::types::masp::PaymentAddress;

// just above main()

fn get_shielded_addr(w: &Wallet<FsWalletUtils>, val: &String) -> PaymentAddress {
	let s = w.find_payment_addr(val).map(|addr| addr.to_string()).unwrap();
	PaymentAddress::from_str(&*s)
}
```

...and that allows us to get the shielded address for the TARGET:

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
let mut xfer = sdk.new_transfer(	
    TransferSource::ExtendedSpendingKey(sk),
    TransferTarget::PaymentAddress(target),
    token.clone(),
    InputAmount::Unvalidated(amt),
);
```

## Transaction Fees

On normal blockchains it is typical for the account that signs the transaction (the one that holds the funds being sent) to pay network fees.  We can do that for shielded transactions but it makes the processing more costly (on my workstation the length of time increased from ~3min to ~13min) as fees have to be paid from a transparent address and we thus force the system to first unshield the fee -- in essence performing a second transaction

Alternatively we can pay the fees from a transparent account, which may make sense for your app if you collect fees from users anyway as a portion of these can then be allocated to paying network fees for your users

Our code can thus behave in one of two ways.  Let's indicate that via an environment variable.  Add the following line to `main()` right after the call to `dotenv()`:

```rust
let fee_payer = std::env::var("FEEPAYER").unwrap();
```

and we'll need to (optionally) pick up the public key for any account passed in.  Add this somewhere before the creation of the SDK object:

```rust
let pk = wallet.find_public_key(&fee_payer);
```

...and now we can add the following lines after creating the transfer object:

```rust
if pk.is_ok() {
    xfer.clone().signing_keys(vec![pk.unwrap()]);
} else {
    xfer.tx.disposable_signing_key = true;
    xfer.tx.fee_unshield = Some(TransferSource::ExtendedSpendingKey(sk));
}
```

so that if a pay account is specified, we add its public key to the list of signing keys for the transaction, and if it's not, we indicate to the SDK that it should use a disposable signing key and we provide the extended spending key for the sender

We can now run our test using the *donor* account from the previous tutorial (which should have 9 NAM), or by using the sender to pay for the fees like this:

```bash
FEEPAYER=donor cargo run # uses the transparent payer account
```
or
```bash
FEEPAYER= cargo run # uses the sender account to pay fees
```

> When using the sender account to pay for fees, if you find your transactions failing, it may have to do with an issue where the VP (validity predicate) rejects the transaction because it was started in one epoch but ended in another
> 
> The configuration in the Campfire chain we're running defines epochs to be 1s long, however we should set that to match the value used in the [Shielded Expedition](https://namada.net/blog/the-namada-shielded-expedition) chain, which is 12 hours
> 
> To change that value edit the `config/genesis/parameters.toml` file, found in the directory where you cloned the *namada-selfhost* project, as follows (in my setup I made epochs 10s long):
> 
> ```toml
> epochs_per_year = 730
> ```
>
> ...and restart the chain
> ```bash
> docker compose restart
> ```

« [prev](../simple/README.md) | [next](../IBC/README.md) »
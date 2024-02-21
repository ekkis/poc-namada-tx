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
let mut xfer = sdk.new_transfer(	
    TransferSource::ExtendedSpendingKey(sk),
    TransferTarget::PaymentAddress(target),
    token.clone(),
    InputAmount::Unvalidated(amt),
);
```

But there's one more thing: typically the account that signs the transaction (the one that holds funds being sent) also pays for the transaction fees.  We can do that for shielded transactions as shown below but it makes the processing more costly as it requires the system to perform 2 transactions:

```rust
xfer.tx.disposable_signing_key = true;
xfer.tx.fee_unshield = Some(TransferSource::ExtendedSpendingKey(sk));
```

> if you find your transactions failing, it may have to do with an issue where the VP (validity predicate) rejects the transaction because it was started in one epoch and ended in another
> 
> The Namada chain defines epochs to be 1s long, however that can be reconfigured by editing the `config/genesis/parameters.toml` file, found in the directory where you cloned the *namada-selfhost* project, as follows (in my setup I made epochs 10s long):
> 
> ```toml
> epochs_per_year = 3_153_600
> ```

## Paying Fees From a Transparent Account

In Cosmos, there's an alternative to paying fees from the sender's account: you can designate a *transparent* account as the payee of the fees

This may make sense for your app if your app collects fees from users, as it can then allocate some portion of those funds to pay for user transactions

It also makes transactions faster (on my workstation the length of time reduced to ~3min from ~13min)

To accomplish that, let's use the *donor* account created in the previous tutorial and replace the transfer call with the following:

```rust
let pk = wallet.find_public_key("donor")
    .expect("Unable to find key");

let mut xfer = sdk.new_transfer(
    TransferSource::ExtendedSpendingKey(sk),
    TransferTarget::PaymentAddress(target),
    token.clone(),
    InputAmount::Unvalidated(amt),
)
.signing_keys(vec![pk]);
```

Running the test:

```bash
$ time cargo run
```

should produce output similar to this:

> Compiling namada-poc v0.1.0 (/Users/.../tx/shielded)
> Finished dev [unoptimized + debuginfo] target(s) in 16.49s
> Running `target/debug/namada-poc`

> sent: true
> tx: EF5ED5F1EF697EA5BB0DF9508FFC97025AC363668990B45A708A802EDDF98187

> real	3m5.870s
> user	4m58.349s
> sys	0m3.427s


> 

« [prev](../simple/README.md) | [next](../IBC/README.md) »
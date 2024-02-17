# My First Namada Transaction

In this document I will walk you through coding your first Namada transaction

To be able to follow, you'll need some basic infrastructure.  Please refer to the [SETUP](SETUP.md) file before getting started and everything is set up, let's create a new project (I've chosen to call it _poc-namada-tx_ but you can call it what you like):

```bash
cargo new poc-namada-tx && cd $_
```

Now we need to include a few "crates" (libraries of functionality) that we'll need in our programme. We do this by editing the `Cargo.toml` file and making sure the `[dependencies]` section contains the following:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
dotenvy = "0.15.7"
```

Let's now edit the `src/main.rs` making it look like this:
```rust
use dotenvy::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok(); // read environment file
}
```

A word of explanation: the _main()_ function in Rust programmes cannot normally be declared as asynchronous, which means it's not able to call functions that return _futures_ (as _promises_ are called in Rust). We fix that via the use of a runtime engine, which is what the decorator `#[tokio::main]` does

Also, to facilitate state we'll keep needed information in an environment file. The `dotenvy` crate provides functionality for reading such files. Naturally then, we'll also need to create a `.env` file. We do this as follows:

```bash
cat <<+ > .env
RPC=http://127.0.0.1:26657
CHAIN_ID=$chain_id
TOKEN=nam
SOURCE=donor
TARGET=charity
AMOUNT=1000000
+
```
Please note that the values for `CHAIN_ID` was computed earlier and `SOURCE` and `TARGET` refer to the 2 keypairs we created earlier, whilst `TOKEN` is the name of the token we funded _donor_ with.

The environment file can now be be loaded into the programme but to use it we need one more step: create a `src/config.rs` with a struct that can hold that data:
```rust
#[derive(clap::Parser, Debug)]
pub struct AppConfig {
    #[clap(long, env)]
    pub rpc: String,

    #[clap(long, env)]
    pub chain_id: String,
    
    #[clap(long, env)]
    pub token: String,
    
    #[clap(long, env)]
    pub source: String,
    
    #[clap(long, env)]
    pub target: String,
    
    #[clap(long, env)]
    pub amount: u64,
}
```
…make it available for use:

```bash
echo "pub mod config;" > src/lib.rs
```

…add the crate below to the Cargo.toml which facilitates parsing:

```toml
clap = { version = "4.4.2", features = ["derive", "env"] }
```

…and now we can load these values into a config variable.  Amend the `src/main.rs` file as follows:

```rust
// add these imports to the top of the file

use clap::Parser;
use std::{sync::Arc};
use namada_poc::{config::AppConfig};

// and call within main()

let config = Arc::new(AppConfig::parse());
```

Try running `cargo run` for it to compile!

## Connecting to the validators

Next we'll try connecting to our local chain. Let's include the Namada SDK and the Tendermint RPC library in the `Cargo.toml` file:
```toml
namada_sdk = { git = "https://github.com/anoma/namada", tag = "v0.30.0", default-features = false, features = ["tendermint-rpc", "std", "async-client", "async-send", "download-params", "rand"] }
tendermint-rpc = { version = "0.34.0", features = ["http-client"] }
```
and import and use it in the code:
```rust
// top of the file

use std::str::FromStr;
use tendermint_rpc::{HttpClient, Url};

// call within main()

let url = Url::from_str(&config.rpc).expect("invalid RPC address");
let http_client = HttpClient::new(url).unwrap();
```

The first line converts the string we extracted from the environment file into a proper _url_ object, issuing an error if the string is not validly formatted, and uses that url to create an HTTP client. This client will later be given to our SDK object to call on

Next we load the wallet we created using the CLI:

```rust
// top of the file

use namada_sdk::wallet::fs::FsWalletUtils;

// call within main()

let basedir = "Library/Application Support/Namada";
let basedir = format!("{}/{}/{}", std::env::var("HOME").unwrap(), basedir, &config.chain_id);

let mut wallet = FsWalletUtils::new(basedir.into());
wallet.load().expect("Failed to load wallet");

```
...and we create a shielded context for our transactions:

```rust
// top of the file

use namada_sdk::masp::fs::FsShieldedUtils;

// call within main()

let shielded_ctx = FsShieldedUtils::new("masp".into());
```

Next we need to get the addresses for the keypairs listed in the environment file.  We use a little function to find them in the wallet:

```rust
// top of the file

use namada_sdk::core::types::{
  address::Address
};

// just before main()

fn get_address(w: &Wallet<FsWalletUtils>, val: &String) -> Address {
	let s = w.find_address(val).map(|addr| addr.to_string()).unwrap();
	Address::decode(s).unwrap()
}
```

...and we grab the addresses for the NAM token, _charity_ account:

```rust
// call within main()

let token = get_address(&wallet, &config.token);
let target = get_address(&wallet, &config.target);
```

For the _donor_ account we need the _spending_ key (since we need to sign the transaction) and its address:

```rust
// within main()

let sk = wallet.find_secret_key(config.source.clone(), None)
	.expect("Unable to find key");
let source = Address::from(&sk.ref_to());
```

We can now create an SDK object to use in building our transaction:
```rust
// top of the file

use namada_sdk::{
    NamadaImpl, io::NullIo,
    args::TxBuilder // facilitates the .chain_id() call
};

// call in main()

let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, NullIo)
  .await
  .expect("unable to initialize Namada context")
  .chain_id(ChainId::from_str(&config.chain_id).unwrap());
```

The above hands us an object to access our SDK that is connected to the chain specified in the environment file, with access to our local wallet and a context for shielding transactions

Having handed the wallet to the SDK object, we no longer need it so we drop it:

```rust
drop(sdk.wallet.write().await);
```

We now denominate the amount to transfer, which includes proper designation of the token to transfer, and the amount involved.  Please note that the amount for the test is expressed in the environment file in cents (NAM tokens are divisible to 6 digits), therefore to transfer 1 NAM token we must indicate to the denominator function 1 x 10^6:

```rust
// top of the file

use namada_sdk::rpc;

// call in main()

let amt = rpc::denominate_amount(
    sdk.client(),
    sdk.io(),
    &token,
    config.amount.into(),
).await;
```

…which we can now build a transaction for:

```rust
// top of the file

use namada_sdk::{
    args::InputAmount,
    core::types::masp::{
        TransferSource, TransferTarget
    }
};

// call in main()

let mut transfer_tx_builder = sdk.new_transfer( 
    TransferSource::Address(source),
    TransferTarget::Address(target.clone()),
    token.clone(),
    InputAmount::Unvalidated(amt),
);
```

and (nicely!) we can add arbitrary text data to the transaction, which means we could include information relevant to a payment (like a delivery address) without revealing that information to the world but only to the recipient:

```rust
let memo = String::from("{\"deliver-to\": \"101 Main Street, Lalaland, CA 91002\"}");
transfer_tx_builder.tx.memo = Some(memo.as_bytes().to_vec());
```

and now we can, finally, build the transaction, sign it and broadcast it to the network:

```rust
// top of file

use namada_sdk::signing::default_sign;

// call in main()

let (mut transfer_tx, signing_data, _epoch) = transfer_tx_builder
    .build(&sdk)
    .await
    .expect("unable to build transfer");
    
sdk.sign(
    &mut transfer_tx,
    &transfer_tx_builder.tx,
    signing_data,
    default_sign,
    (),
)
.await
.expect("unable to sign reveal pk tx");
```

Now we submit the transaction to the network and process the response, printing the status of the transaction and its hash to the console:

```rust
// top of file

use namada_sdk::tendermint::abci::Code;

// call in main()

let process_tx_response = sdk
  .submit(transfer_tx, &transfer_tx_builder.tx)
  .await;

let (sent, tx_hash) = if let Ok(response) = process_tx_response {
        match response {
            namada_sdk::tx::ProcessTxResponse::Applied(r) => (r.code.eq(&ResultCode::Ok), Some(r.hash)),
            namada_sdk::tx::ProcessTxResponse::Broadcast(r) => {
                (r.code.eq(&Code::Ok), Some(r.hash.to_string()))
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    };

// display the transaction hash

print!("sent: {}", sent);
print!("tx: {}", tx_hash.unwrap());
```
The above should compile and run, performing a test transaction

---

# Support

The [official documentation](https://docs.namada.net/) for Namada is available for anyone to read but to learn more and build interesting things, having access to the Namada community is invaluable. Fortunately, you'll find an active community on [Discord](https://discord.gg/namada), where you'll also find me lurking (as @ekkis).  I'm also available on X/Telegram (same username)

Additionally, the entire code base for this article may be found on my Github repo [poc-namada-tx](https://github.com/ekkis/poc-namada-tx), which you can grab like this:
```bash
git clone https://github.com/ekkis/poc-namada-tx.git
```

# Conclusion

Software construction is never easy and certainly the complexity of building on decentralised platforms is dizzying. However, the choice of Rust as a language (and the richness of structures it provides) and Cosmos (a well architected ecosystem) help greatly in achieving functionality that wouldn't have been possible even a few years ago

If you are a developer, it's a great time to be involved and certainly the crypto world is the cutting edge. I look forward to seeing zero-knowledge technology permeate the blockchain ecosystem in the same way that the EFF's HTTPS Everywhere³ campaign did the internet

---

## Footnotes

1. Cross-chain token transfers are accomplished using the Axelar infrastructure, an IBC (inter-blockchain communications) protocol implementation for Cosmos. This allows your tokens to travel across to any Cosmos blockchain, but even to Ethereum and other chains via bridges
2. cf. https://www.eff.org/https-everywhere

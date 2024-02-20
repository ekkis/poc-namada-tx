# Namada Transactions

In this document I will walk you through coding a series of use cases for Namada transactions

The project is structured to accomplish a number of goals, which build on top of each other as follows:

1. [Setup](docs/Setup.md) - Before you get started you'll need some basic infrastructure.  Read and follow this tutorial first and when everything is set up, come back to the next step
2. [Simple](simple/README.md) - Here we create our first plain vanilla transaction.  This does a great deal of the heavy lifting for the other tutorials
3. [Shielded](shielded/README.md) - This tutorial enhances the previous code to perform shielded transactions
4. [IBC](IBC/README.md) - Here we do a shielded transaction with an IBC token instead of a native token
5. [Osmosis](Osmosis/README.md) - With IBC support in the code, we now perform a swap on an Osmosis liquidity pool

There are also a number code snippets I'd like to share:

* Creating accounts in code (includes revealing public keys)

	// let bal = rpc::get_token_balance(&http_client, &token, &source).await;
	// println!("bal (nam)={:?}", bal);

# Support

The [official documentation](https://docs.namada.net/) for Namada is available for anyone to read but to learn more and build interesting things, having access to the Namada community is invaluable. Fortunately, you'll find an active community on [Discord](https://discord.gg/namada), where you'll also find me lurking (as @ekkis).  I'm also available on X/Telegram (same username)

Additionally, the entire code base for this article may be found on my Github repo [poc-namada-tx](https://github.com/ekkis/poc-namada-tx), which you can grab like this:
```bash
git clone https://github.com/ekkis/poc-namada-tx.git
```

# Conclusion

Software construction is never easy and certainly the complexity of building on decentralised platforms is dizzying. However, the choice of Rust as a language (and the richness of structures it provides) and Cosmos (a well architected ecosystem) help greatly in achieving functionality that wouldn't have been possible even a few years ago

If you are a developer, it's a great time to be involved and certainly the crypto world is the cutting edge. I look forward to seeing zero-knowledge technology permeate the blockchain ecosystem in the same way that the EFF's HTTPS Everywhere campaign did the internet

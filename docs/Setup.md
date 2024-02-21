# Namada Walkthroughs

To be able to follow our walkthroughs, a compiler, a virtualisation platform, a package manager and other infrastructure is needed

## Basic Infrastructure

The following base components are required:

* Apple's OS/X operating system. This recipe is known to work on Ventura 13.6.1 so if your OS version differs, you mileage may vary
* the [Homebrew](https://brew.sh) package manager (see link for install instructions)
* [Docker Desktop](https://www.docker.com/products/docker-desktop/) - a virtual machine host that makes our process significantly easier. Whilst not strictly necessary (you can compile a node from code or even get binaries), Docker allows us to run a testnet hassle-free and the process works flawlessly
* You'll also need git, which you can install by installing the command-line tools for [XCode](https://developer.apple.com/xcode/), or can be installed like this:
```bash
brew install git
```

## Running a Local Chain (in Docker containers)

The first thing we need to do is run a local chain we can connect to. For the sake of this document, we'll run [Campfire](https://knowabl.notion.site/Campfire-testnet-5e4c1df53ab64b818a55bfcf36ccc550), one of a number of testnets available, which runs as a collection of Docker containers, thanks to the Namada [Selfhost](https://github.com/0x4r45h/namada-selfhost) project. First let's grab it from Github as shown below:

```bash
git clone https://github.com/0x4r45h/namada-selfhost.git
cd namada-selfhost
```

The project comes with a sample configuration file which must be renamed for use.  Please note that for this recipe to work, the version of Namada the orchestrator runs and this recipe is known to work with is set as shown below:

```bash
ver="v0.31.0"
sed -e "s/^NAMADA_TAG=.*/NAMADA_TAG=$ver/" .env.sample > .env
```

We now let the orchestrator run the validator nodes for us:

```bash
docker compose pull   # loads all the images (may take a little while)
docker compose up -d  # runs the node
cd ..                 # don't forget this!
```

and, if you look in the Docker Desktop app will look something like:

![image](poc-namada-tx-docker.png)

The chain will take a little while to come up (but it's far faster than running a public network which make take a much as 20 hours to sync) and you can check its status with the command below. When the result is `false`, you're ready to go!

```bash
function nm-status() {
    curl -s http://127.0.0.1:26657/status |jq .result
}
nm-status |jq -r .sync_info.catching_up
```

> Incidentally, you'll notice we decided to encapsulate use of the CLI into functions.  This allows us greater flexibility and brevity of expression in these tutorials, and we conveniently provide you with a library of these functions for your use in the `.namada` file in the root of this project, which you can source from your profile script like this:
> ```bash
> echo ". $PWD/.namada" >> ~/.bash_profile
> ```

If you don't have `jq` installed, Brew can do it for you:

```bash
brew install jq
```

## Installing Local Binaries

For development purposes we cannot work inside the containers, so having binaries locally installed makes sense. Let's grab these from Github² and put them into a directory in our path (they must be the same version as that run by the orchestrator):

```bash
loc="https://github.com/anoma/namada/releases/download/$ver/namada-$ver-Darwin-x86_64.tar.gz"
curl -L $loc |tar xzvf -
cp $(echo $loc |sed 's/.*\///; s/\.tar\.gz$//')/namada* /usr/local/bin
```

now make sure you've got the right version:

```bash
namada --version
```

## Joining the Chain

We can now connect our binaries to the local validators by joining the local chain. The _chain id_ is handily provided to us by a service running on the first validator on port 8123 (this port number is internal to the container but it's mapped to a local port by Docker, which the code below looks up)

```bash
# encapsulate

export NAMADA_NODE=namada-selfhost-namada-1-1
nm-port() {
    docker ps -f name=$NAMADA_NODE --format 'json {{.Ports}}' |perl -ne '/:(\d+)->'$1'/; print $1'
}

nm-chain() {
    curl -sL $1 |perl -ne 'print $1 if /href="(.*?)\.tar.gz'
}

# fetch chain-id from the configuration service

srv="http://127.0.0.1:$(nm-port 8123)/"
chain_id=$(nm-chain $srv)

# join the network

export NAMADA_NETWORK_CONFIGS_SERVER=$srv
namada client utils join-network --chain-id $chain_id
```

The above will create a directory at `~/Library/Application Support/Namada` containing genesis files and connection information for your chain. In my case, the directory is called `local.73532805d0f0897a687825d1`

## Developer Setup

Functionality on Namada is built on Rust, so you'll need the compiler installed. If you don't already have it, install it like this:

```bash
brew install rust
```

## Upgrading Your Infrastructure

As the network upgrades, you can keep up easily thanks to the [namada-selfhost](https://medium.com/r/?url=https%3A%2F%2Fgithub.com%2F0x4r45h%2Fnamada-selfhost) project. To upgrade should be as easy as (from the folder where you cloned the project):
```bash
git pull               # perform in the folder where the project was cloned
docker compose down -v # the -v removes old volumes
docker compose up -d   # restart services
```

However, don't forget to also update your CLI binaries and the SDK.  Also, restarting the chain generates a new chain id, so make sure you join the new chain with the CLI and that your apps know the new _chain_id_

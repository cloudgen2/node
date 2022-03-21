Riochain 3.0 is based on [Astar](./README_Astar.md)

## Development

### use ci-cd runner build
1) Start node with default parameters
    > docker run --rm -p 80:80 public.ecr.aws/e6s7r2u6/rio-node-public:<YOUR_JOB_ID>
2) Start node with custom parameters
    > docker run --rm -p 80:80 public.ecr.aws/e6s7r2u6/rio-node-public:<YOUR_JOB_ID> --chain dev2 --tmp
3) Check the node had started and accepting RPC calls
    > curl -H "Content-Type: application/json" --data '{ "jsonrpc":"2.0", "method":"system_health", "params":[],"id":1 }' localhost:80
    
    You should see the output: `{"jsonrpc":"2.0","result":{"isSyncing":false,"peers":0,"shouldHavePeers":false},"id":1}`                                                     

### build locally and run
```shell script
docker-compose up --build
```

### stop node
```shell script
docker-compose down
```

### attach to container
```shell script
docker-compose exec rio-node bash
```

### build spec
...todo `docker-compose exec rio-node /home/ubuntu/src/target/collator build-spec --dir=/tmp`

### use the latest build from AWS-ECR
```shell script
aws configure
aws ecr get-login-password --region eu-central-1 | docker login --username AWS --password-stdin 061416964074.dkr.ecr.eu-central-1.amazonaws.com
docker pull 061416964074.dkr.ecr.eu-central-1.amazonaws.com/rio-node:latest
```

### Install AWS client
1) Install aws cli using [official instructions](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
2) Login to AWS public container registry
    > aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws

# System Requirements.

## System requirements for validator nodes
* CPU 4000MHz 12 cores.
* RAM 64 Gb.
* The archive node has 256 Gb disks per year.

## Small nodes or clients (not consensus members) that have access to history records.
* CPU 2000MHz 4 cores.
* RAM 8Gb.
* Disk 8Gb.

### Embedded documents.
After the project is built, you can use the following commands to explore all the parameters and
Subcommand:
``sh
./target/release/rio-node -h
``

##Operating parameters

Run the development chain from the cargo

``sh
Cargo run --release - --dev --tmp
``

Clear any existing development chain status:

``sh
./target/release/rio-node purge-chain --dev
``

Starting from the log:

``sh
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/rio-node -lruntime=debug --dev
``

### Multi-node local test network

[Start private network tutorial](https://substrate.dev/docs/en/tutorials/start-a-private-network/).

## Default port.
* 9933 is HTTP
* 9944 is WS
* 9615 is Prometheus
* 30333 p2p flow

# Configure parameters.

## Command Line.

### Choose a chain.

Use the ```--chain <chainspec>``` option to select the chain. It can be local, development, staging, testing, or custom chain specification.

### Specify a custom base path (storage path)
Flags ```-d```, ```--base-path <PATH>```

### Specify the boot node list
Mark ```--bootnodes <ADDR>...```

### RPC port.
Use the ```--rpc-external``` flag to expose the RPC port, and use ```--ws-external`` to expose the websockets. Not all RPC calls are safe, and you should use an RPC proxy to filter out insecure calls. Use the ```--rpc-port``` and ```--ws-port``` options to select the port. To limit the hosts that can be accessed, use the ```--rpc-cors``` option.

### Export block.
To export blocks to a file, use export-blocks. Export as JSON (default) or binary (```--binary true```).
```polkadot export block --from 0 <output_file>```

### Workers under the chain.
Use the ```--offchain-worker``` flag to set whether offchain worker should be executed on each block
By default, it is only enabled for nodes that are creating new blocks.
[Default value: WhenValidating] [Possible values: Always, Never, WhenValidating]

### Validator node in non-archive mode.
Add the following flag: ```--unsafe-pruning --pruning <NUM OF BLOCKS>```, a reasonable value is 1000. Note that the databases of the archive node and the non-archive node are not compatible with each other, and to switch, you need to clear the chain data.

### Archive node.
The archive node will not prune any block or status data. Use the ```--pruning archive``` flag. Certain types of nodes (such as validators) must run in archive mode. Likewise, all events in each block are cleared from the state, so if you want to store events, then you will need an archive node.

Note: By default, the validator node is in archive mode. If you have synchronized the chain in non-archive mode, you must first delete the database using polkadot purge-chain, and then make sure to run Polkadot with the ```--pruning=archive``` option.

### Run a temporary node.
Sign ```--tmp```

A temporary directory will be created to store the configuration and will be deleted at the end of the process.

Note: The directory executed by each process is random. This directory is used as the base path, which includes: database, node key, and keystore.

###Node Status
You can check the health of a node through RPC using the following command:
``
curl -H "Content-Type: application/json" --data'{ "jsonrpc":"2.0", "method":"system_health", "params":[],"id":1 }'localhost:9933
``

### External documentation.
* [Alice and Bob start the blockchain](https://substrate.dev/docs/en/tutorials/start-a-private-network/alicebob)
* [Node Management](https://wiki.polkadot.network/docs/en/build-node-management)

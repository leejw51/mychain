# Changelog

*December 17, 2019*

The release is a more complete alpha version meant to be deployed on the second iteration of the public testnet.
There are no guarantees on future API and binary compatibility at this stage.

## v0.1.0

### Features
* *chain-abci*: early punishment logic -- validators that do not maintain liveness or exhibit Byzantine faults are punished 
by having their respective account states jailed and their associated stake slashed. Slashing amount
depends on the configurable network parameter as well as how many validators are being punished at the same time. Unjailing can happen manually using a special transaction type (UnjailTx).
* *chain-abci*: early reward distribution logic -- fees and emissions (using an exponential decay) from the initial rewards pool are periodically distributed to validators.
* *chain-abci*: dynamic validator set -- new validator nodes can join the validator set by sending a special transaction type (NodeJoinTx).
* *chain-abci*: more complete and flexible genesis configuration -- the genesis file now contains the network parameters guarding the punishment and rewards logic, and the initial distribution is more flexible (e.g. amounts can be unbonded from a custom time instead of a genesis time).
* *chain-tx-enclave*: transaction workflows -- basic flows around transaction data confidentiality are sketched out in the client and validation and query enclaves (broadcasted transactions are encrypted with a static mock key; client can connect to an enclave over a remotely attested TLS connection and query transactions that were previously sealed on that node).
* *client*: HD-wallet -- using the standardized BIP44 derivations and BIP39 mnemonics.
* *client*: early lite verification -- [block-level lite client verification](https://github.com/tendermint/tendermint/blob/master/docs/architecture/adr-044-lite-client-with-weak-subjectivity.md) using an early tendermint-rs implementation and basic app hash checking

### Bug Fixes
* [365](https://github.com/crypto-com/chain/pull/365) Creating a transfer address also creates a staking address
* a lot of others, but as we didn't maintain this changelog, it'd take ages to list them all :/

### Improvements
* quite a few improvements in client, e.g. not signing every intermediate transaction used in fee estimation 

### Breaking changes
* *client*: internals changed a lot: many dependency bumps + a different storage schema
* *chain-abci*: app state format changes in genesis.json + storage format 
* *chain-tx-enclave*: changes in enclave-protocol and in EDL

### Known Limitation
* (compile-time) static mock key for transaction obfuscation
* many other things (e.g. full node synchronization needs to happen from genesis, no UTXO commitments)

*September 12, 2019*

A quick bug fix on top of 0.0.2

## v0.0.3

### Bug fixes
* [373](https://github.com/crypto-com/chain/pull/373) client-index auto-sync didn't work correctly

*September 11, 2019*

A small patches (mainly dependency bumps) to the released version

## v0.0.2

### Features
* [360](https://github.com/crypto-com/chain/pull/360) command to remove a wallet from client-rpc auto-sync

### Bug fixes
* [368](https://github.com/crypto-com/chain/pull/368) client-cli can select the network via an environment variable for address display

### Improvements
* [364](https://github.com/crypto-com/chain/pull/364) fewer logs in client-rpc


*September 6, 2019*

The release is an incomplete alpha version meant to be deployed on the first iteration of the public testnet.
There are no guarantees on future API and binary compatibility at this stage.

## v0.0.1

### Features
* initial genesis procedure based on one-off snapshotting the ERC20 contract state and several allocation mentioned in the original whitepaper
* initial rewards pool
* initial configurable linear fee scheme
* initial basic network operations related to bonded stake management
* sketched out obfuscated transaction format
* transaction binary payloads use [SCALE](https://github.com/paritytech/parity-scale-codec#parity-scale-codec) codec
* transaction validation in enclaves isolated in a separate process reached via a 0MQ socket
* threshold multi-signature support using Merkle trees of combined public keys and the Schnorr MuSig scheme
* * (for the sample use case in the context of what was marketed as "Proof of Goods & Services Delivered", see https://github.com/crypto-com/multisig-demo ) 
* client library "backend" support for transfers, multi-signatures and staking operations

### Known Limitation
Far too many to list them all (e.g. a validator set fixed at genesis or temporarily mocked transaction privacy) :)

*Pre-alpha versions*

While versioning was not strictly followed in the pre-alpha stages, there were a few tagged releases that signalled format changes that broke the client functionality:

* `pre-alpha-feeless-client`: the change after this tag added the linear fee scheme; the client could have potentially construct invalid transactions due to not accounting for the fee
* `pre-alpha-no-account-genesis`: before staking operations support, the initial "state" was constructed from transactions with no inputs; the change after this tag added the extra staking-related operations
* `pre-alpha-pre-enc-tx`: the change after this tag sketched out the obfuscated transaction format

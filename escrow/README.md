# Escrow

A tiny contract to maintain a registry of accounts in escrow. This contract will be 100% owner-free, meaning escrow contract can never be updated or rug-pulled. Escrow will deploy a new Deed contract for every account being transferred.

### Transfer Flows

#### General Workflow

1. Create new escrow contract, only needs deploy once
2. Deploy new Deed contract for an account
3. Close Escrow for a Deed

Optional other actions:

- Revert Title: Allow owner to regain ownership via escrow
- Encumbrance: Allow escrow to remove any straggling access keys

#### Initialization

This happens upon contract deploy of escrow contract. Main requirement is allowing the escrow contract to keep a registry of escrowed accounts.

1. Auction House deploys new escrow contract, without access keys
2. Users verify no access keys via `near keys escrow.nym.near`

#### Deploy Deed

Deploys a contract to an account which should be transferred. Only allowed to be executed once, and is done only from account owner.

#### Revert Title

Proxy to deed revert_ownership function

#### Encumbrance

Proxy to deed remove_key function

#### Close Escrow

Proxy to deed transfer_ownership function

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/escrow.wasm --initFunction new --initArgs '{}' --accountId escrow_account.testnet

# deploy deed
near call _escrow_account_ deed '{"title": "some_account.testnet"}' --accountId youraccount.testnet

# Cancel deed
near call _escrow_account_ revert_title '{"title": "some_account.testnet"}' --accountId youraccount.testnet

# Remove Access Key
near call _escrow_account_ encumbrance '{"title": "some_account.testnet", "key": "ed25591:PK_HERE"}' --accountId youraccount.testnet

# Close deed
near call _escrow_account_ close_escrow '{"title": "some_account.testnet", "new_key": "ed25591:PK_HERE"}' --accountId youraccount.testnet

```
# Escrow

A tiny contract to maintain a registry of accounts in escrow. This contract will be 100% owner-free, meaning escrow contract can never be updated or rug-pulled. Escrow will deploy a new Deed contract for every account being transferred.

### Transfer Flows

#### General Workflow

1. Create new escrow contract, only needs deploy once
2. Deploy new Deed contract for an account
3. Close Escrow for a Deed

Optional other actions:

- Revert Title: Allow owner to regain ownership via escrow

#### Initialization

This happens upon contract deploy of escrow contract. Main requirement is allowing the escrow contract to keep a registry of escrowed accounts.

1. Registrar or DAO deploys new escrow contract, without access keys
2. Users verify no access keys via `near keys escrow.nym.near`

#### Register Title

Register a new title by the following workflow (under the hood):
1. Deploy a new Deed contract to title account (not this contract)
2. call the register method so escrow knows to escrow the deed
3. optionally call a registrar so it also knows the escrowed account

#### Clear Escrow

Proxy to deed change_ownership function

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/escrow.wasm --initFunction new --initArgs '{"factory": "testnet", "registrar": "auction.nym.testnet", "dao": "dao.sputnik.testnet"}' --accountId escrow_account.testnet

# Start deed (but this is actually called at DEED deploy)
near call _escrow_account_ register '{"underwriter": "some_other_account.testnet", "registrar": true}' --accountId youraccount_to_auction.testnet

# Close deed
near call _escrow_account_ close_escrow '{"auction_id": "some_account.testnet", "new_key": "ed25591:PK_HERE"}' --accountId youraccount.testnet

# Update Settings (only via DAO)
near call _escrow_account_ update_settings '{"dao": "dao.sputnik.testnet", "registrar": "registrar.alias.testnet"}' --accountId dao.sputnik.testnet

# view if account is in escrow
near view _escrow_account_ in_escrow '{"title": "some_account.testnet"}'

# view the escrow settings
near view _escrow_account_ get_settings

```
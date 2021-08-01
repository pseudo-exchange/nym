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

1. Auction House deploys new escrow contract, without access keys
2. Users verify no access keys via `near keys escrow.nym.near`

#### Register Title

Register a new title by the following workflow (under the hood):
1. delete the account, to clear out all access keys
2. create the same acount again, assigning escrow as owner
3. deploy a deed contract to new account, assigning original owner to have rights to revert ownership via escrow proxy.

#### Clear Escrow

Proxy to deed transfer_ownership function

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

```
# Escrow

A tiny contract to manage account ownership and transfer functionality, by utilizing an escrow witness as the coordinator

### Transfer Flows

#### General Workflow

1. 

#### Initialization

This happens upon contract deploy from escrow contract. This requires the user to grant escrow public key access to their account. All precautions must be taken before initialization can proceed.

1. 

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/account_manager.wasm --initFunction new --initArgs '{}' --accountId escrow_account.testnet

# transfer ownership
near call _account_here_ transfer_ownership '{}' --accountId youraccount.testnet

```
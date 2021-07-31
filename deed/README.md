# Deed

A tiny contract to manage account ownership and transfer functionality, by utilizing an escrow contract as the coordinator

### Transfer Flows

#### General Workflow

1. Initialization
2. change ownership only possible upon auction finalized by new owner

#### Initialization

This happens upon contract deploy from escrow contract. This requires the user to grant escrow full access to their account. All precautions must be taken before initialization can proceed.

1. User grants access to escrow, adding full access key to the account being transfered
2. User deploys a Deed contract, and calls the function "new" on the newly deployed Deed contract, which executes the following logic:
  2A. Make sure the initialization is valid
  2B. Transfer any/all balance on this account (No ability to know any sub account balances)
  2C. Call escrow to register this account with it, optionally allow escrow to tell registrar to include this account in its registery for THIS underwriter
  2D. Finish by assigning state, for future contract calls

#### Transfer of Ownership

The escrow contract will manage the movement from escrow ownership into the new access keys. This is done by allowing the escrow to make judgements about whether an account is available and ready to make a transfer. Such logic and caveats can be seen in the escrow folder.

1. Check access
2. Add new full access key, the new owner
3. Remove original owner id

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/deed.wasm --initFunction new --initArgs '{"escrow": "escrow.testnet", "underwriter": "ACCOUNT_THAT_WILL_OWN.testnet"}' --accountId ACCOUNT_THAT_WILL_OWN.testnet --gas 300000000000000

# transfer ownership (Only callable via escrow)
near call escrow.testnet claim '{"pk": "ed25519:..."}' --accountId ACCOUNT_THAT_OWNS.testnet --gas 300000000000000
```
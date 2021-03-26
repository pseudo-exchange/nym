# Deed

A tiny contract to manage account ownership and transfer functionality, by utilizing an escrow witness as the coordinator

### Transfer Flows

#### General Workflow

1. Initialization
2. IF too many keys -- Remove Keys until only escrow
3. IF owner changes mind -- Revert Ownership
4. Transfer ownership

#### Initialization

This happens upon contract deploy from escrow contract. This requires the user to grant escrow public key access to their account. All precautions must be taken before initialization can proceed.

1. User grants access to escrow, adding full access key to the account being transfered
2. Escrow contract deploys a Deed contract, the code in this folder
3. Escrow calls the function "new" on the newly deployed Deed contract, which executes the following logic:
  3A. Make sure the initialization is valid
  3B. Transfer any/all balance on this account (No ability to know any sub account balances)
  3C. Finish by assigning state, for future contract calls

#### Removing Keys

In the event that escrow system determines there are more access keys than are needed, escrow will use this function to fully clear any remaining keys. This function will be executed directly at the consent of the user, as they are in fact paying to have this done. It is necessary to maintain the risk by forcing the user to pay in this case.

1. User gets a warning in a web UI showing there are more full access keys to remove
2. User signs a transaction to remove access key, which isnt allowed to remove escrow, only all other access keys.

#### Reverting Ownership

At some point, an owner will decide they no longer want to leave an account in escrow. They can utilize this function to revert the ownership to the original access keys. This does not however, allow the user to revert to any new access keys, and if the originals are lost, a transfer via escrow MUST be made.

1. Check that only escrow can call, and txn is valid
2. Add full access key back to known original owner
3. Remove escrow full access key

#### Transfer of Ownership

The escrow contract will manage the movement from escrow ownership into the new access keys. This is done by allowing the escrow to make judgements about whether an account is available and ready to make a transfer. Such logic and caveats can be seen in the escrow folder.

1. Check that access & txn are valid
2. Add new full access key, the new owner
3. Remove escrow full access key

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/deed.wasm --initFunction new --initArgs '{"escrow_pk": "ed25591:PK_HERE", "escrow_account_id": "account_to_transfer.testnet", "original_owner_pk": "ed25591:PK_HERE"}' --accountId escrow_account.testnet

# transfer ownership
near call _account_here_ transfer_ownership '{"new_owner_pk": "ed25591:PK_HERE"}' --accountId youraccount.testnet

# revert ownership
near call _account_here_ revert_ownership '{}' --accountId youraccount.testnet

# remove key
near call _account_here_ remove_key '{"remove_key": "ed25591:PK_HERE"}' --accountId youraccount.testnet

```
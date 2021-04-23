# Transfer Owner

A tiny contract to fully revoke all previous access keys, to enable a trustless account transfer

### General Workflow

1. Deploy contract
2. Allow only a specified predecessor to call function
3. Function to remove account -- Dangerous!

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/transfer_owner.wasm --initFunction new --initArgs '{"escrow": "escrow_account.testnet", "beneficiary": "otheraccount.testnet"}' --accountId YOUR_ACCOUNT.testnet

# transfer ownership
near call _account_here_ delete_self --accountId escrow_account.testnet
```
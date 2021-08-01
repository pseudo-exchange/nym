#!/bin/bash
# Uncomment the desired network
export NEAR_ENV=testnet
# export NEAR_ENV=mainnet

export FACTORY=testnet
# export FACTORY=near
# export FACTORY=registrar

export MASTER_ACCOUNT=nym.testnet
export UNDERWRITER_ACCOUNT_ID=underwriter_bash.$MASTER_ACCOUNT
export ESCROW_ACCOUNT_ID=escrow_bash.$MASTER_ACCOUNT
export REGISTRAR_ACCOUNT_ID=registrar_bash.$MASTER_ACCOUNT
export CRON_ACCOUNT_ID=cron.in.testnet
export DAO_ACCOUNT_ID=dao.sputnikv2.testnet

export TITLE_ACCOUNT_ID=acct00000001.testnet
export TITLE_PK=ed25519:6Mzi9dRMSiPWYp7BgLJ2Lj6KPCcs48FwB93NgQ4LKSBo

# Deploy the deed to escrow
near deploy --wasmFile res/deed.wasm --initFunction new --initArgs '{"escrow": "'$ESCROW_ACCOUNT_ID'", "underwriter": "'$UNDERWRITER_ACCOUNT_ID'"}' --accountId $TITLE_ACCOUNT_ID --initGas 300000000000000 --initDeposit 1

# Check escrow has it
near view $ESCROW_ACCOUNT_ID in_escrow '{"title": "'$TITLE_ACCOUNT_ID'"}'

# Register new auction
near call $REGISTRAR_ACCOUNT_ID create '{"title": "'$TITLE_ACCOUNT_ID'"}' --accountId $UNDERWRITER_ACCOUNT_ID --amount 2 --gas 300000000000000
# , "auction_close_block": 41000000, "is_blind": true

# Check registrar has it
near view $REGISTRAR_ACCOUNT_ID get_auction_by_id '{"id": "'$TITLE_ACCOUNT_ID'"}'

echo "Auction Created"
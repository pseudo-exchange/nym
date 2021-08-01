#!/bin/bash
# This file is used for starting a fresh set of all contracts & configs
set -e

if [ -d "res" ]; then
  echo ""
else
  mkdir res
fi

cd "`dirname $0`"

if [ -z "$KEEP_NAMES" ]; then
  export RUSTFLAGS='-C link-arg=-s'
else
  export RUSTFLAGS=''
fi

# build the things
cargo build --all --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

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

# create all accounts
near create-account $UNDERWRITER_ACCOUNT_ID --masterAccount $MASTER_ACCOUNT
near create-account $ESCROW_ACCOUNT_ID --masterAccount $MASTER_ACCOUNT
near create-account $REGISTRAR_ACCOUNT_ID --masterAccount $MASTER_ACCOUNT

# Deploy all the contracts to their rightful places
near deploy --wasmFile res/escrow.wasm --initFunction new --initArgs '{"factory": "'$FACTORY'", "registrar": "'$REGISTRAR_ACCOUNT_ID'", "dao": "'$DAO_ACCOUNT_ID'"}' --accountId $ESCROW_ACCOUNT_ID
near deploy --wasmFile res/registrar.wasm --initFunction new --initArgs '{"escrow": "'$ESCROW_ACCOUNT_ID'", "dao": "'$DAO_ACCOUNT_ID'", "cron": "'$CRON_ACCOUNT_ID'"}' --accountId $REGISTRAR_ACCOUNT_ID

# Create dummy account(s)
near call $FACTORY create_account '{"new_account_id": "'$TITLE_ACCOUNT_ID'", "new_public_key" :"'$TITLE_PK'"}' --accountId $MASTER_ACCOUNT --amount 5 --gas 300000000000000

echo "Setup Complete"
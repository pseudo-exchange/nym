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

# clear and recreate all accounts
near delete $UNDERWRITER_ACCOUNT_ID $MASTER_ACCOUNT
near delete $ESCROW_ACCOUNT_ID $MASTER_ACCOUNT
near delete $REGISTRAR_ACCOUNT_ID $MASTER_ACCOUNT
near delete $TITLE_ACCOUNT_ID $MASTER_ACCOUNT

echo "Clear Complete"
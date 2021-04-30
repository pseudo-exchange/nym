# Registrar

A tiny contract to manage account ownership and transfer functionality, by utilizing an escrow account as the coordinator

### Transfer Flows

#### General Workflow

1. Deploy registrar
2. Create an auction for account (that is in escrow)
3. accept bids between auction blocks
4. Optional: Reveal phase
5. Finalize auction

#### Initialization

This happens upon contract deploy. Used to specify the escrow account

1. 

### Commands & Usage

Requires [near cli]()

```bash
# Init
near deploy --wasmFile res/registrar.wasm --initFunction new --initArgs '{}' --accountId registrar_account.testnet


```
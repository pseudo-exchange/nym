use near_sdk::{
    near_bindgen,
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{ ValidAccountId },
    env,
    Promise,
    PanicOnDefault,
};

near_sdk::setup_alloc!();
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TransferOwner {
    escrow: ValidAccountId,
    beneficiary: ValidAccountId,
    underwriter: ValidAccountId,
}

/// TransferOwner
/// A tiny contract to fully revoke all previous access keys, to enable a trustless account transfer
#[near_bindgen]
impl TransferOwner {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// All funds and previous ownership MUST be removed before escrow can validate
    /// the account is available for any other ownership transfers
    ///
    /// near deploy --wasmFile res/transfer_owner.wasm --initFunction new --initArgs '{"escrow": "escrow_account.testnet", "beneficiary": "otheraccount.testnet"}' --accountId YOUR_ACCOUNT.testnet
    #[init(ignore_state)]
    pub fn new(escrow: ValidAccountId, beneficiary: ValidAccountId, underwriter: ValidAccountId) -> Self {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Signer must be able to relinquish ownership");
        assert_ne!(env::signer_account_id(), escrow.to_string(), "Escrow must not be original owner");
        assert_ne!(env::current_account_id(), underwriter.to_string(), "Signer cannot be current account");

        TransferOwner {
            escrow,
            beneficiary,
            underwriter
        }
    }

    /// Called only by the escrow contract, this allows the escrow to fully capture
    /// the account, only having a single account for safer deletions
    ///
    /// NOTE: While this will technically work if you own escrow, it will happen in mainnet via cross-contract call
    /// near call _account_here_ delete_self --accountId escrow_account.testnet
    pub fn delete_self(&self) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow.to_string(), "Unauthorized access, escrow only");
        assert_eq!(env::signer_account_id(), self.underwriter.to_string(), "Signer must be the underwriter");

        // The scariest part of the code is also the simplest :)
        Promise::new(env::current_account_id())
            .delete_account(self.beneficiary.to_string())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::{test_utils::{accounts, VMContextBuilder}};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    use super::*;

    fn create_blank_account_manager(a: usize, b: usize) -> TransferOwner {
        TransferOwner::new(
            accounts(a),
            accounts(b)
        )
    }

    // escrow: Acct 1
    // signer: Acct 2
    fn get_context(predecessor_account_id: ValidAccountId, idx: Option<usize>) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(idx.unwrap_or(0)))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    #[should_panic(expected = "Signer must be able to relinquish ownership")]
    fn test_init_fail() {
        let context = get_context(accounts(1), Some(usize::from(u8::from(0))));
        testing_env!(context.build());
        create_blank_account_manager(1, 3);
    }

    #[test]
    #[should_panic(expected = "Escrow must not be original owner")]
    fn test_init_escrow_fail() {
        let context = get_context(accounts(1), Some(usize::from(u8::from(1))));
        testing_env!(context.build());
        create_blank_account_manager(1, 1);
    }

    #[test]
    fn test_init() {
        let context = get_context(accounts(0), Some(usize::from(u8::from(0))));
        testing_env!(context.build());
        let contract = create_blank_account_manager(1, 3);
        assert_eq!(contract.escrow, accounts(1));
        assert_eq!(contract.beneficiary, accounts(3));
    }

    #[test]
    #[should_panic(expected = "Unauthorized access, escrow only")]
    fn test_delete_self_error() {
        let context = get_context(accounts(0), Some(usize::from(u8::from(0))));
        testing_env!(context.build());
        let contract = create_blank_account_manager(1, 3);

        contract.delete_self();
    }

    #[test]
    fn test_delete_self() {
        let context = get_context(accounts(0), Some(usize::from(u8::from(0))));
        testing_env!(context.build());
        let contract = TransferOwner::new(
            accounts(1),
            accounts(3)
        );

        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(1))
            .signer_account_id(accounts(1).clone())
            .signer_account_pk(b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec())
            .predecessor_account_id(accounts(1));
        testing_env!(builder.build());

        contract.delete_self();
    }
}
use near_sdk::{
    near_bindgen,
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{ ValidAccountId, Base58PublicKey },
    env,
    Promise,
    PublicKey,
    PanicOnDefault,
};

near_sdk::setup_alloc!();

#[ext_contract]
pub trait ExtEscrow {
    fn register(&mut self);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Deed {
    escrow_account_id: ValidAccountId,
    escrow_pk: PublicKey,
    underwriter_id: ValidAccountId,
    underwriter_pk: PublicKey,
}

/// Deed
/// A deployable contract to an account that needs witness-based access transfer
#[near_bindgen]
impl Deed {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// the account is available for any other ownership transfers
    #[init(ignore_state)]
    pub fn new(
        underwriter_id: ValidAccountId,
        underwriter_pk: Base58PublicKey,
        escrow_account_id: ValidAccountId,
        escrow_pk: Base58PublicKey
    ) -> Self {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Signer must be able to relinquish ownership");

        Deed {
            escrow_account_id,
            escrow_pk: escrow_pk.into(),
            underwriter_id,
            underwriter_pk: underwriter_pk.into()
        }
    }

    /// Allows original owner to get access back to this account
    /// This function is only accessible via escrow, but can be called by anyone
    pub fn revert_ownership(&mut self) -> Promise {
        self.change_ownership(self.underwriter_pk.clone())
    }

    /// Completely changes the access keys of this account
    pub fn transfer_ownership(&mut self, new_underwriter_pk: Base58PublicKey) -> Promise {
        self.change_ownership(new_underwriter_pk.into())
    }

    /// Internal function for adding/removing access keys
    fn change_ownership(&mut self, pk: PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        // Add new access key
        // Remove escrow key
        Promise::new(env::current_account_id())
            .add_full_access_key(pk)
            .delete_key(self.escrow_pk.clone().into())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::convert::TryFrom;
    use near_sdk::{test_utils::{accounts, VMContextBuilder}};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    use super::*;

    fn create_blank_account_manager() -> Deed {
        Deed::new(
            accounts(1),
            Base58PublicKey::try_from("ed25519:Ggs1UC1zJpa1K11Q33H7KtA5TQ6ik7YoRpTyv3nkoA9o".to_string()).unwrap(),
            accounts(0),
            Base58PublicKey::try_from("ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_string()).unwrap()
        )
    }

    // escrow: Acct 0
    // signer: Acct 1
    // transfer: Acct 2
    fn get_context(c: ValidAccountId, s: ValidAccountId, p: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(c)
            .signer_account_id(s)
            .predecessor_account_id(p);
        builder
    }

    #[test]
    fn test_init() {
        let context = get_context(accounts(1), accounts(1), accounts(0));
        testing_env!(context.build());
        let contract = create_blank_account_manager();
        assert_eq!(contract.escrow_account_id, accounts(0));
    }

    #[test]
    fn test_transfer_ownership() {
        let context = get_context(accounts(1), accounts(1), accounts(0));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager();

        contract.transfer_ownership(Base58PublicKey::try_from("ed25519:2mXmCTrFHMYTBv2kUEKGrKwk1wdT5EfXmFL85P6Xr9dV".to_string()).unwrap());

        assert_eq!(contract.escrow_account_id, accounts(0));
    }

    #[test]
    fn test_revert_ownership() {
        let context = get_context(accounts(1), accounts(1), accounts(0));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager();

        contract.revert_ownership();

        assert_eq!(contract.escrow_account_id, accounts(0));
    }
}
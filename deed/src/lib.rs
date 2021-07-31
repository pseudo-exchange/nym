use near_sdk::{
    AccountId,
    near_bindgen,
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{ ValidAccountId, Base58PublicKey },
    env,
    Promise,
    PromiseResult,
    PanicOnDefault,
    log,
};

near_sdk::setup_alloc!();

const REGISTER_GAS_FEE: u64 = 20_000_000_000_000; // 20 Tgas
const CALLBACK_GAS_FEE: u64 = 20_000_000_000_000; // 20 Tgas

#[ext_contract(ext_escrow)]
pub trait ExtEscrow {
    fn register(&mut self);
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn ownership_callback(&mut self, original_owner: AccountId);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Deed {
    escrow: AccountId,
    underwriter: AccountId,
}

/// Deed
/// A deployable contract to an account that needs witness-based access transfer
#[near_bindgen]
impl Deed {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// the account is available for any other ownership transfers
    ///
    /// ```bash
    /// near deploy --wasmFile res/transfer_owner.wasm --initFunction new --initArgs '{"escrow": "escrow.testnet", "underwriter": "ACCOUNT_THAT_WILL_OWN.testnet"}' --accountId ACCOUNT_THAT_WILL_OWN.testnet --gas 300000000000000
    /// ```
    #[init(ignore_state)]
    pub fn new(
        underwriter: ValidAccountId,
        escrow: ValidAccountId
    ) -> Self {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Signer must have original ownership");
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Signer must have original ownership");

        // register with escrow contract
        ext_escrow::register(
            &escrow.to_string(),
            0,
            REGISTER_GAS_FEE,
        );

        Deed {
            escrow: escrow.to_string(),
            underwriter: underwriter.to_string(),
        }
    }

    /// Adding access keys for escrow mediated keys
    /// IMPORTANT: pk MUST be the pk of the claimer's signing keys, otherwise they wont be able to own it!
    ///
    /// ```bash
    /// near call escrow.testnet claim '{"pk": "ed25519:..."}' --accountId ACCOUNT_THAT_OWNS.testnet --gas 300000000000000
    /// ```
    pub fn change_ownership(&mut self, pk: Base58PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow.to_string(), "Unauthorized access, escrow only");

        // Remove underwriter so escrow is the sole executor of the account temporarily
        self.underwriter = AccountId::default();

        // Add new access key
        Promise::new(env::current_account_id())
            .add_full_access_key(pk.into())
            .then(
                ext_self::ownership_callback(
                    self.underwriter.clone(),
                    &env::current_account_id(),
                    0,
                    CALLBACK_GAS_FEE,
                )
            )
    }

    /// Internal function to check that the key change was successful
    #[private]
    pub fn ownership_callback(&mut self, original_owner: AccountId) {
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                // NOTE: this contract could be removed now.
                log!("Owner transfer success");
            }
            PromiseResult::Failed => {
                // reset owner if unsuccessful
                self.underwriter = original_owner;
                log!("Owner transfer failure");
            }
            PromiseResult::NotReady => unreachable!(),
        };
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
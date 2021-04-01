use near_sdk::{
    near_bindgen,
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde_json::{json},
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
    owner_pk: PublicKey
}

/// Deed
/// A deployable contract to an account that needs witness-based access transfer
#[near_bindgen]
impl Deed {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// All funds and previous ownership MUST be removed before escrow can validate
    /// the account is available for any other ownership transfers
    #[init(ignore_state)]
    pub fn new(
        underwriter: ValidAccountId,
        escrow_account_id: ValidAccountId,
        escrow_pk: Base58PublicKey
    ) -> Self {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Signer must be able to relinquish ownership");
        let owner_pk = env::signer_account_pk();

        // Escrow needs to know about this somehow, ext register fn?
        // ext_escrow::register(&escrow_account_id, 0, env::prepaid_gas() / 3);
        Promise::new(escrow_account_id.to_string())
            .function_call(
                b"register".to_vec(),
                json!({
                    "underwriter": underwriter,
                }).to_string().as_bytes().to_vec(),
                env::attached_deposit(),
                env::prepaid_gas() / 2
            );

        Promise::new(env::current_account_id())
            .delete_key(owner_pk.clone().into());

        Deed {
            escrow_account_id,
            escrow_pk: escrow_pk.into(),
            owner_pk: owner_pk.into()
        }
    }

    /// Allows original owner to get access back to this account
    /// This function is only accessible via escrow, but can be called by anyone
    pub fn revert_ownership(&mut self) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        Promise::new(env::current_account_id())
            .add_full_access_key(self.owner_pk.clone())
    }

    /// Called only by the escrow contract, this allows the escrow to fully capture
    /// the account, only having a single key for ownership
    pub fn remove_key(&mut self, remove_key: PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        Promise::new(env::current_account_id())
            .delete_key(remove_key)
    }

    /// Completely changes the access keys of this account
    pub fn transfer_ownership(&mut self, new_owner_pk: PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        self.owner_pk = new_owner_pk.clone().into();

        // Add new owner key
        // Remove escrow key
        Promise::new(env::current_account_id())
            .add_full_access_key(new_owner_pk.into())
            .delete_key(self.escrow_pk.clone().into())
    }
}

// TODO: Test - ownership transfer is successful, reverting can only happen by original owner
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::{test_utils::{accounts, VMContextBuilder}};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    use super::*;

    fn create_blank_account_manager(context: VMContextBuilder) -> Deed {
        Deed::new(
            accounts(1),
            context.context.signer_account_pk
        )
    }

    // escrow: Acct 1
    // signer: Acct 2
    // transfer: Acct 3
    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_init() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = create_blank_account_manager(context);
        assert_eq!(contract.escrow_account_id, accounts(1));
    }

    #[test]
    fn test_transfer_ownership() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager(context);

        contract.transfer_ownership(b"ed25519:DDysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec());

        assert_eq!(contract.escrow_account_id, accounts(1));
        assert_eq!(contract.owner_pk, b"ed25519:DDysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec());
    }

    #[test]
    fn test_revert_ownership() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager(context);

        contract.revert_ownership();

        assert_eq!(contract.escrow_account_id, accounts(1));
    }

    #[test]
    // #[should_panic(expected = "Cannot remove escrow")]
    fn test_remove_key() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager(context);

        contract.remove_key(b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec());
    }
}
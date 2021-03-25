use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ ValidAccountId };
use near_sdk::{
    env, Promise, near_bindgen, setup_alloc, PanicOnDefault, PublicKey
};

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AccountManager {
    escrow_account_id: ValidAccountId,
    owner_pk: PublicKey,
    pk: PublicKey
}

/// AccountManager
/// A deployable contract to an account that needs witness-based access transfer
#[near_bindgen]
impl AccountManager {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// All funds and previous ownership MUST be removed before escrow can validate
    /// the account is available for any other ownership transfers
    #[init]
    pub fn new(
        escrow_pk: PublicKey,
        escrow_account_id: ValidAccountId,
        original_owner_pk: PublicKey
    ) -> Self {
        assert_ne!(env::signer_account_id(), env::current_account_id(), "Cannot sign against current account");
        assert_ne!(&escrow_pk, &original_owner_pk, "Cannot use same keys");

        // Transfer out remaining value?
        let p1 = Promise::new(env::signer_account_id())
            .transfer(env::account_balance());

        let p2 = Promise::new(env::current_account_id())
            .add_full_access_key(escrow_pk.clone().into())
            .delete_key(original_owner_pk.clone().into());

        p1.then(p2);

        AccountManager {
            escrow_account_id,
            owner_pk: original_owner_pk.into(),
            pk: escrow_pk.into()
        }
    }

    /// Allows original owner to get access back to this account
    /// This function is only available to the original owner
    pub fn revert_ownership(&mut self) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");
        assert_eq!(self.owner_pk, env::signer_account_pk(), "Invalid Owner");

        Promise::new(env::current_account_id())
            .add_full_access_key(self.owner_pk.clone())
            .delete_key(self.pk.clone())
    }

    /// Called only by the escrow contract, this allows the escrow to fully capture
    /// the account, only having a single key for ownership
    pub fn remove_key(&mut self, remove_key: PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");
        let rmky = remove_key.into();
        assert_ne!(rmky, self.pk, "Cannot remove escrow");

        Promise::new(env::current_account_id())
            .delete_key(rmky)
    }

    /// Completely changes the access keys of this account
    pub fn transfer_ownership(&mut self, new_owner_pk: PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        self.owner_pk = new_owner_pk.clone().into();

        Promise::new(env::current_account_id())
            .add_full_access_key(new_owner_pk.into())
            .delete_key(self.pk.clone())
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

    fn create_blank_account_manager(context: VMContextBuilder) -> AccountManager {
        AccountManager::new(
            b"ed25519:3tysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec(),
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
    fn test_remove_key() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = create_blank_account_manager(context);

        contract.remove_key(b"ed25519:BtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec());

        assert_eq!(contract.escrow_account_id, accounts(1));
    }
}
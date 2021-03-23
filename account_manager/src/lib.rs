use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58PublicKey};
use near_sdk::{
    env, Promise, near_bindgen, setup_alloc, PanicOnDefault, PublicKey
};

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AccountManager {
    owner_pk: PublicKey,
    pk: PublicKey
}

// TODO: AccountManager
// This contract gets deployed by escrow.nym.near
// Has a function to revert control back to original owner
// Has a function to change owner, callable ONLY by escrow.nym.near
#[near_bindgen]
impl AccountManager {
    #[init]
    pub fn new(escrow_pk: Base58PublicKey, original_owner_pk: Base58PublicKey) -> Self {
        AccountManager {
            owner_pk: original_owner_pk.into(),
            pk: escrow_pk.into()
        }
    }

    pub fn revert_ownership(&mut self) -> Promise {
        // TODO: Add asserts
        Promise::new(env::current_account_id())
            .add_full_access_key(self.owner_pk)
            .delete_key(self.pk)
    }

    pub fn transfer_ownership(&mut self, new_owner_pk: Base58PublicKey) -> Promise {
        // TODO: Add asserts
        Promise::new(env::current_account_id())
            .add_full_access_key(new_owner_pk.into())
            .delete_key(self.pk)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    use super::*;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_thang() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = AccountManager::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.thang(), "hiii");
    }
}
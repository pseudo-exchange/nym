use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ ValidAccountId, Base58PublicKey };
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

// TODO: AccountManager
// This contract gets deployed by escrow.nym.near
// Has a function to revert control back to original owner
// Has a function to change owner, callable ONLY by escrow.nym.near
#[near_bindgen]
impl AccountManager {
    /// Upon deploy, contract initializes with only escrow account owning this account
    /// All funds and previous ownership MUST be removed before escrow can validate
    /// the account is available for any other ownership transfers
    #[init]
    pub fn new(
        escrow_pk: Base58PublicKey,
        escrow_account_id: ValidAccountId,
        original_owner_pk: Base58PublicKey
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
    pub fn remove_key(&mut self, remove_key: Base58PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");
        let rmky = remove_key.into();
        assert_ne!(rmky, self.pk, "Cannot remove escrow");

        Promise::new(env::current_account_id())
            .delete_key(rmky)
    }

    // TODO: Deploy a new contract over this one to complete the transfer of ownership?
    pub fn transfer_ownership(&mut self, new_owner_pk: Base58PublicKey) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.escrow_account_id.to_string(), "Unauthorized access, escrow only");

        Promise::new(env::current_account_id())
            .add_full_access_key(new_owner_pk.into())
            .delete_key(self.pk.clone())
    }
}

// TODO: Test - ownership transfer is successful, reverting can only happen by original owner
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
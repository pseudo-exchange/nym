use near_sdk::{
    ext_contract,
    near_bindgen,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde_json::{json},
    collections::{ LookupSet },
    json_types::{ ValidAccountId },
    AccountId,
    env,
    log,
    Promise,
    PublicKey,
    PanicOnDefault,
};

near_sdk::setup_alloc!();

const ESCROW_STORAGE_KEY: [u8; 1] = [0];

#[ext_contract]
pub trait ExtDeed {
    fn new(&mut self, escrow_account_id: ValidAccountId, original_owner_pk: PublicKey);
    fn revert_ownership(&mut self) -> Promise;
    fn remove_key(&mut self, remove_key: PublicKey) -> Promise;
    fn transfer_ownership(&mut self, new_owner_pk: PublicKey) -> Promise;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Escrow {
    // THIS account id, most cases: escrow.nym.near
    id: AccountId,

    // keeps track of the escrowed accounts
    accounts: LookupSet<AccountId>
}

// Contract keeps track of accounts in escrow
// is the only account that can execute functions on escrowed account
// should not be owned by anyone -- NO ACCESS KEYS!
#[near_bindgen]
impl Escrow {
    #[init(ignore_state)]
    pub fn new() -> Self {
        Escrow {
            id: env::current_account_id(),
            accounts: LookupSet::new(ESCROW_STORAGE_KEY.to_vec())
        }
    }

    /// Responsible for bonding an account to a deed contract, where
    /// escrow is the sole owner, and can only transfer ownership upon
    /// close of title
    pub fn deed(&mut self, title: AccountId) -> Promise {
        // Make sure this account isnt already in escrow
        // assert_ne!(self.accounts.contains(&title.to_string()), true, "Account already in escrow");

        // add to registry
        // self.accounts.insert(&title.to_string());
        log!("New deed: {}", &title);

        // deploy deed
        let p1 = Promise::new(title.clone())
            .deploy_contract(
                include_bytes!("../../res/deed.wasm").to_vec()
            );

        let p2 = Promise::new(title.to_string())
            .function_call(
                b"new".to_vec(),
                json!({
                    "escrow_account_id": env::current_account_id(),
                    "original_owner_pk": env::signer_account_id()
                }).to_string().as_bytes().to_vec(),
                env::attached_deposit(),
                env::prepaid_gas() / 2
            );

        p1.then(p2)
    }

    /// Allows an owner to cancel a deed, given appropriate parameters
    pub fn revert_title(&mut self, title: ValidAccountId) -> Promise {
        assert_eq!(self.accounts.contains(&title.to_string()), true, "Account not in escrow");

        // Remove from registry
        self.accounts.remove(&title.to_string());

        // Call the deed, to revert title back to owner
        ext_deed::revert_ownership(
            &title,
            0,
            env::prepaid_gas() / 3
        )
    }

    /// Removes any excess public keys on an account, to ensure full
    /// Account transfer can happen trustlessly
    pub fn encumbrance(&self, title: ValidAccountId, key: PublicKey) -> Promise {
        assert_eq!(self.accounts.contains(&title.to_string()), true, "Account not in escrow");

        // Call the deed, to remove a key
        ext_deed::remove_key(
            key,
            &title,
            0,
            env::prepaid_gas() / 3
        )
    }

    /// The full realization of an escrow deed, where the account is
    /// transferred to the new owner
    pub fn close_escrow(&mut self, title: ValidAccountId, new_key: PublicKey) -> Promise {
        assert_eq!(self.accounts.contains(&title.to_string()), true, "Account not in escrow");

        // Remove from registry
        self.accounts.remove(&title.to_string());
        log!("Close deed: {}", &title);

        // Call the deed, to transfer ownership to new public key
        ext_deed::transfer_ownership(
            new_key,
            &title,
            0,
            env::prepaid_gas() / 3
        )
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
        let contract = Escrow::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.thang(), "hiii");
    }
}
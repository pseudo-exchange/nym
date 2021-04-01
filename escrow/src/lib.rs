use near_sdk::{
    ext_contract,
    near_bindgen,
    borsh::{self, BorshDeserialize, BorshSerialize},
    // serde_json::{json},
    collections::{ LookupMap },
    json_types::{ ValidAccountId, Base58PublicKey },
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
    fn new(&mut self, escrow_account_id: ValidAccountId);
    fn revert_ownership(&mut self) -> Promise;
    fn remove_key(&mut self, remove_key: PublicKey) -> Promise;
    fn transfer_ownership(&mut self, new_owner_pk: PublicKey) -> Promise;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Escrow {
    // THIS account id, most cases: escrow_nym.near
    id: AccountId,

    // TODO: Owner -- auction house?
    owner_id: Option<AccountId>,

    // keeps track of the escrowed accounts
    accounts: LookupMap<AccountId, AccountId>
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
            accounts: LookupMap::new(ESCROW_STORAGE_KEY.to_vec()),
            owner_id: None,
        }
    }

    /// Responsible for bonding an account to a deed contract, where
    /// escrow is the sole owner, and can only transfer ownership upon
    /// close of title
    // NOTE: Currenly only possible if this escrow account has a public key with full access to account, otherwise deploy is not possible.
    pub fn register(&mut self, underwriter: ValidAccountId) {
        let title = env::signer_account_id();
        // Make sure this account isnt already in escrow
        assert_ne!(self.accounts.contains_key(&title.to_string()), true, "Account already in escrow");

        // add to registry
        self.accounts.insert(&title.to_string(), &underwriter.to_string());
        log!("New deed: {}", &title);
    }

    /// Allows an owner to cancel a deed, given appropriate parameters
    pub fn revert_title(&mut self, title: AccountId) -> Promise {
        self.is_in_escrow(title.clone());
        self.is_underwriter(title.clone());

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
    pub fn encumbrance(&self, title: AccountId, key: Base58PublicKey) -> Promise {
        self.is_in_escrow(title.clone());
        self.is_underwriter(title.clone());

        // Call the deed, to remove a key
        ext_deed::remove_key(
            key.into(),
            &title,
            0,
            env::prepaid_gas() / 3
        )
    }

    /// The full realization of an escrow deed, where the account is
    /// transferred to the new owner
    pub fn close_escrow(&mut self, title: AccountId, new_key: PublicKey) -> Promise {
        self.is_in_escrow(title.clone());
        // TODO: Can only be called by auction house

        // Remove from registry
        self.accounts.remove(&title.to_string());
        log!("Close deed: {}", &title);

        // Call the deed, to transfer ownership to new public key
        ext_deed::transfer_ownership(
            new_key.into(),
            &title,
            0,
            env::prepaid_gas() / 3
        )
    }

    fn is_in_escrow(&self, title: AccountId) {
        assert_eq!(self.accounts.contains_key(&title.to_string()), true, "Account not in escrow");
    }

    fn is_underwriter(&self, title: AccountId) {
        assert_eq!(self.accounts.get(&title.to_string()).unwrap(), env::signer_account_id(), "Account cannot control escrow account");
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
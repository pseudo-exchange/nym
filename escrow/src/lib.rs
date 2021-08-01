use near_sdk::{
    ext_contract,
    near_bindgen,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{ LookupMap },
    json_types::{ ValidAccountId, Base58PublicKey },
    AccountId,
    env,
    log,
    Promise,
    BorshStorageKey,
    PanicOnDefault,
    StorageUsage,
};

near_sdk::setup_alloc!();

// TODO: Finalize amounts needed!
// Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa - Batmannnnnnnn
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
const CLOSE_ESCROW_GAS_FEE: u64 = 50_000_000_000_000; // 50 Tgas

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
    Tlas,
}

#[ext_contract(ext_deed)]
pub trait ExtDeed {
    fn new(underwriter: ValidAccountId, escrow: ValidAccountId, registrar: Option<ValidAccountId>) -> Self;
    fn change_ownership(&mut self, pk: Base58PublicKey) -> Promise;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Escrow {
    pub base_storage_usage: StorageUsage,

    /// The account that can create base accounts
    /// testnet factory: "testnet"
    /// mainnet factory: "near"
    /// mainnet TLA factory: "registrar"
    pub factory: AccountId,

    /// The account that handles all logic for auctions, bids & other actions
    pub registrar: AccountId,

    // keeps track of the escrowed accounts
    tlas: LookupMap<AccountId, AccountId>,
    accounts: LookupMap<AccountId, AccountId>,

    // Optional
    pub dao: Option<AccountId>,

    // TODO: setup DAO whitelists and params for TLAs
}

/// Escrow
/// Contract keeps track of accounts in escrow
/// is the only account that can execute functions on escrowed account
/// should not be owned by anyone -- NO ACCESS KEYS!
#[near_bindgen]
impl Escrow {
    /// Initialize an escrow instance
    /// NO migration logic is to be implementated, as this contract should not have any full access keys
    ///
    /// ```bash
    /// near deploy --wasmFile res/escrow.wasm --initFunction new --initArgs '{"factory": "testnet", "registrar": "auction.nym.testnet", "dao": "dao.sputnik.testnet"}' --accountId escrow_account.testnet
    /// ```
    #[init]
    pub fn new(
        factory: ValidAccountId,
        registrar: ValidAccountId,
        dao: Option<AccountId>,
    ) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        assert_eq!(env::current_account_id(), env::predecessor_account_id(), "Must be called by owner");

        let mut this = Escrow {
            base_storage_usage: 0,
            factory: factory.to_string(),
            registrar: registrar.to_string(),
            tlas: LookupMap::new(StorageKeys::Tlas),
            accounts: LookupMap::new(StorageKeys::Accounts),
            dao,
        };
        // compute storage needs before finishing
        this.measure_account_storage_usage();
        this
    }

    /// Measure the storage an agent will take and need to provide
    fn measure_account_storage_usage(&mut self) {
        let initial_storage_usage = env::storage_usage();
        // Create a temporary, dummy entry and measure the storage used.
        let tmp_account_id = "z".repeat(64);
        self.accounts.insert(&tmp_account_id, &tmp_account_id);
        self.base_storage_usage = env::storage_usage() - initial_storage_usage;
        // Remove the temporary entry.
        self.accounts.remove(&tmp_account_id);
    }

    /// Responsible for bonding an account to a deed contract, where
    /// escrow is the sole owner, and can only transfer ownership upon
    /// close of title
    ///
    /// ```bash
    /// near call _escrow_account_ register '{"underwriter": "some_other_account.testnet"}' --accountId youraccount_to_auction.testnet
    /// ```
    ///
    #[payable]
    pub fn register(&mut self, underwriter: AccountId) {
        let acct = env::predecessor_account_id();
        // Make sure this account isnt already in escrow
        assert_ne!(self.accounts.contains_key(&acct), true, "Account already in escrow");

        // Store the account in escrow
        self.accounts.insert(&acct, &underwriter);
        log!("Account {} is in escrow", &acct);
    }

    // TODO: support TLAs
    /// The full realization of an escrow deed, where the account is
    /// transferred to the new owner OR the old owner.
    /// If the account was registered with registrar, then we must check the signer.
    ///
    /// near call _escrow_account_ close_escrow '{"title": "some_account.testnet", "new_key": "ed25591:PK_HERE"}' --accountId youraccount.testnet
    /// ```
    pub fn close_escrow(&mut self, title: ValidAccountId, new_key: Base58PublicKey) -> Promise {
        let acct_id = title.clone().to_string();
        let acct = self.accounts.get(&acct_id).expect("Account is not in escrow");

        // Check that this is indeed the owner
        if self.registrar == env::predecessor_account_id() {
            assert_eq!(acct, env::signer_account_id(), "Account does not control deed account");
        } else {
            assert_eq!(acct, env::predecessor_account_id(), "Account does not control deed account");
        }

        // Remove from registry
        self.accounts.remove(&acct_id);
        log!("Close deed: {}", &acct_id);

        // Call the deed, to transfer ownership to new public key
        ext_deed::change_ownership(
            new_key,
            &acct_id,
            0,
            CLOSE_ESCROW_GAS_FEE,
        )
    }

    /// Checks if an account is escrowed
    ///
    /// ```bash
    /// near view _escrow_account_ in_escrow '{"title": "some_account.testnet"}'
    /// ```
    pub fn in_escrow(&self, title: ValidAccountId) -> bool {
        self.accounts.get(&title.to_string()).is_some()
    }

    /// Get the owner for a specific title
    ///
    /// ```bash
    /// near view _escrow_account_ get_underwriter '{"title": "some_account.testnet"}'
    /// ```
    pub fn get_underwriter(&self, title: ValidAccountId) -> Option<AccountId> {
        self.accounts.get(&title.to_string())
    }

    /// Gets the escrow settings
    ///
    /// ```bash
    /// near view _escrow_account_ get_settings
    /// ```
    pub fn get_settings(&self) -> (
        AccountId,
        AccountId,
        Option<AccountId>,
    ) {
        (
            self.registrar.clone(),
            self.factory.clone(),
            self.dao.clone(),
        )
    }

    /// change the contract basic parameters, in case of needing to upgrade
    /// or change to different account IDs later.
    /// Can only be called by the DAO contract (if originally configured)
    ///
    /// ```bash
    /// near call _escrow_account_ update_settings '{"dao": "dao.sputnik.testnet", "registrar": "registrar.alias.testnet"}' --accountId dao.sputnik.testnet
    /// ```
    pub fn update_settings(
        &mut self,
        dao: Option<ValidAccountId>,
        factory: Option<ValidAccountId>,
        registrar: Option<ValidAccountId>,
    ) {
        assert!(self.dao.is_some(), "No ownership, cannot change settings");
        assert_eq!(self.dao.clone().unwrap(), env::predecessor_account_id(), "Callee must be dao contract");
        
        // Update each individual setting
        if dao.is_some() { self.dao = Some(dao.unwrap().to_string()); }
        if factory.is_some() { self.factory = factory.unwrap().to_string(); }
        if registrar.is_some() { self.registrar = registrar.unwrap().to_string(); }
    }

    /// Returns semver of this contract.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    use super::*;

    // factory: Acct 0
    // registrar: Acct 1
    // escrow (me): Acct 2
    // dao: Acct 3
    fn create_blank_escrow() -> Escrow {
        Escrow::new(
            accounts(0),
            accounts(1),
            Some(accounts(3).to_string())
        )
    }

    fn get_context(c: ValidAccountId, s: ValidAccountId, p: ValidAccountId, is_view: Option<bool>) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(c)
            .signer_account_id(s)
            .predecessor_account_id(p)
            .is_view(is_view.unwrap_or(false));
        builder
    }

    #[test]
    fn test_init() {
        let context = get_context(accounts(3), accounts(3), accounts(3), Some(false));
        testing_env!(context.build());
        let contract = create_blank_escrow();
        assert_eq!(contract.factory, accounts(0).to_string());
        assert_eq!(contract.registrar, accounts(1).to_string());
    }

    #[test]
    #[should_panic(expected = "Must be called by owner")]
    fn test_init_fail() {
        let context = get_context(accounts(3), accounts(2), accounts(2), Some(false));
        testing_env!(context.build());
        create_blank_escrow();
    }

    #[test]
    fn test_register() {
        let mut context = get_context(accounts(3), accounts(3), accounts(3), Some(false));
        testing_env!(context.build());
        let mut contract = create_blank_escrow();

        context = get_context(accounts(3), accounts(2), accounts(2), Some(false));
        testing_env!(context.build());

        contract.register(accounts(2).to_string(), Some(true));
    }

    // #[test]
    // #[should_panic(expected = "Account already in escrow")]
    // fn test_register_error() {
    //     let context = get_context(accounts(3), accounts(3), accounts(3));
    //     testing_env!(context.build());
    //     let mut contract = create_blank_escrow();

    //     let context2 = get_context(accounts(3), accounts(2), accounts(2));
    //     testing_env!(context2.build());

    //     contract.register(accounts(2));
    //     testing_env!(context2.build());
    //     contract.register(accounts(2));
    // }

    #[test]
    fn test_in_escrow() {
        let context = get_context(accounts(3), accounts(3), accounts(3), Some(false));
        testing_env!(context.build());
        let mut contract = create_blank_escrow();

        let context2 = get_context(accounts(3), accounts(2), accounts(2), Some(false));
        testing_env!(context2.build());

        contract.register(accounts(2).to_string(), Some(false));

        let context3 = get_context(accounts(3), accounts(2), accounts(2), Some(true));
        testing_env!(context3.build());
        let is_registered: bool = contract.in_escrow(accounts(2));
        assert!(is_registered, "Needs to be registered");
    }

    // #[test]
    // fn test_register() {
    //     let context = get_context(accounts(0), Some(usize::from(u8::from(0))));
    //     testing_env!(context.build());
    //     let contract = TransferOwner::new(
    //         accounts(1),
    //         accounts(3)
    //     );

    //     let mut builder = VMContextBuilder::new();
    //     builder
    //         .current_account_id(accounts(1))
    //         .signer_account_id(accounts(1).clone())
    //         .signer_account_pk(b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec())
    //         .predecessor_account_id(accounts(1));
    //     testing_env!(builder.build());

    //     contract.register();
    // }
}
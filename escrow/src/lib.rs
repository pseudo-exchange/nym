use near_sdk::{
    ext_contract,
    near_bindgen,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde_json::{json},
    collections::{ LookupMap },
    json_types::{ ValidAccountId, Base58PublicKey },
    AccountId,
    env,
    log,
    Promise,
    PromiseResult,
    PromiseOrValue,
    PublicKey,
    PanicOnDefault,
};
use std::convert::TryFrom;

near_sdk::setup_alloc!();

const ESCROW_STORAGE_TLA_KEY: [u8; 1] = [0];
const ESCROW_STORAGE_ACCOUNT_KEY: [u8; 1] = [1];

// TODO: Finalize amounts needed!
// Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa Ⓝa - Batmannnnnnnn
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
// const NEW_ACCOUNT_STORAGE_AMOUNT: u128 = ONE_NEAR * 6;
const T_GAS: u64 = 1_000_000_000_000;
// const MAX_GAS_FEE: u64 = 300 * T_GAS;
// const NEW_ACCOUNT_GAS_FEE: u64 = 25 * T_GAS;
// const DELETE_ACCOUNT_GAS_FEE: u64 = 25 * T_GAS;
// const REGISTER_CALLBACK_GAS_FEE: u64 = 50 * T_GAS;
// const DEED_NEW_GAS_FEE: u64 = 25 * T_GAS;
const DELETE_ACCOUNT_GAS_FEE: u64 = 5_000_000_000_000; // 5 Tgas
const REGISTER_CALLBACK_GAS_FEE: u64 = 20_000_000_000_000; // 20 Tgas
const NEW_ACCOUNT_GAS_FEE: u64 = 100_000_000_000_000;
const DEED_NEW_GAS_FEE: u64 = 100_000_000_000_000;
// const REGISTRAR: &str = "registrar";

#[ext_contract(ext_self)]
trait ExtSelf {
    fn on_register_p1(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise;
    fn on_register_p2(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise;
    fn p2(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise;
    fn p2a(&mut self, title: ValidAccountId) -> Promise;
    fn on_register_p3(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise;
}

#[ext_contract]
trait ExtAccountFactory {
    fn create_account(
        &mut self,
        new_account_id: AccountId,
        new_public_key: Base58PublicKey,
    ) -> Promise;
}

#[ext_contract]
pub trait ExtDeed {
    fn new(underwriter_id: ValidAccountId, underwriter_pk: Base58PublicKey, escrow_account_id: ValidAccountId, escrow_pk: Base58PublicKey) -> Self;
    fn revert_ownership(&mut self) -> Promise;
    fn transfer_ownership(&mut self, new_underwriter_pk: PublicKey) -> Promise;
}

#[ext_contract]
pub trait ExtTransferOwner {
    fn delete_self(&self) -> Promise;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Escrow {
    /// THIS account id, most cases: escrow_nym.near
    id: AccountId,
    pk: PublicKey,

    /// The account that can create base accounts (in testnet, factory_id: "testnet", mainnet factory_id: "near")
    factory_id: AccountId,

    /// The account that handles all logic for auctions, bids & other actions
    auction_id: AccountId,

    // keeps track of the escrowed accounts
    tlas: LookupMap<AccountId, AccountId>,
    accounts: LookupMap<AccountId, AccountId>

    // TODO: setup DAO whitelists and params for TLAs
}

/// Escrow
/// Contract keeps track of accounts in escrow
/// is the only account that can execute functions on escrowed account
/// should not be owned by anyone -- NO ACCESS KEYS!
#[near_bindgen]
impl Escrow {
    /// ```bash
    /// near deploy --wasmFile res/escrow.wasm --initFunction new --initArgs '{"factory_id": "testnet", "auction_id": "auction.nym.testnet", "pk": "escrow_public_key"}' --accountId escrow_account.testnet
    /// ```
    #[init(ignore_state)]
    pub fn new(
        factory_id: ValidAccountId,
        auction_id: ValidAccountId,
        pk: Base58PublicKey
    ) -> Self {
        assert_eq!(env::current_account_id(), env::signer_account_id(), "Must be called by owner");

        Escrow {
            id: env::current_account_id(),
            pk: pk.into(),
            factory_id: factory_id.to_string(),
            auction_id: auction_id.to_string(),
            tlas: LookupMap::new(ESCROW_STORAGE_TLA_KEY.to_vec()),
            accounts: LookupMap::new(ESCROW_STORAGE_ACCOUNT_KEY.to_vec()),
        }
    }

    // TODO: Finish gas needs
    /// Callback of the on_register function
    /// Responsible for initializing the deed contract with ownership params
    #[private]
    pub fn on_register_p3(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise {
        // add to registry
        self.accounts.insert(&title.to_string(), &underwriter.to_string());
        log!("New deed: {}", &title);
        log!("Now im trying to init deed");

        ext_deed::new(
            underwriter.clone(),
            Base58PublicKey::try_from(env::signer_account_pk()).unwrap(),
            ValidAccountId::try_from(self.id.clone()).unwrap(),
            Base58PublicKey::try_from(self.pk.clone()).unwrap(),
            &title.clone().to_string(),
            0,
            DEED_NEW_GAS_FEE
        )
    }

    // TODO: Change to more callbacks, Finish gas needs
    /// Callback of the register function
    /// Deploys a contract to the new account, must be initialed!
    #[private]
    pub fn on_register_p2(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise { //OrValue<bool>
        // if is_promise_success() {
            log!("Now im trying to deploy deed");
            // now that account has been deleted, create again!
            // PromiseOrValue::Promise(
                // deploy deed contract, to allow final management
                Promise::new(title.clone().to_string())
                    .deploy_contract(
                        include_bytes!("../../res/deed.wasm").to_vec(),
                    )
                    .then(
                        ext_self::on_register_p3(
                            title,
                            underwriter,
                            &env::current_account_id(),
                            0,
                            REGISTER_CALLBACK_GAS_FEE 
                        )
                    )
        //     )
        // } else {
        //     // If register failed, dont do anything we have to revert
        //     PromiseOrValue::Value(false)
        // }
    }

    // TODO: Finish gas needs
    /// Callback of the register function
    /// Responsible for creating the account again, bonding to escrow ownership
    #[payable]
    #[private]
    pub fn on_register_p1(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise { //OrValue<bool>  underwriter: ValidAccountId
        // if is_promise_success() {
            log!("Now im trying to create account");

            // now that account has been deleted, create again!
            // PromiseOrValue::Promise(
                ext_account_factory::create_account(
                    title.clone().to_string(),
                    Base58PublicKey::try_from(env::signer_account_pk()).unwrap(),
                    &self.factory_id,
                    env::attached_deposit(),
                    NEW_ACCOUNT_GAS_FEE
                )
                .then(
                    ext_self::on_register_p2(
                        title,
                        underwriter,
                        &env::current_account_id(),
                        0,
                        REGISTER_CALLBACK_GAS_FEE
                    )
                )
        //     )
        // } else {
        //     // If register failed, dont do anything we have to revert
        //     PromiseOrValue::Value(false)
        // }
    }

    // TODO: Finish gas needs
    /// Responsible for bonding an account to a deed contract, where
    /// escrow is the sole owner, and can only transfer ownership upon
    /// close of title
    ///
    /// ```bash
    /// near call _escrow_account_ register '{"underwriter": "some_other_account.testnet"}' --accountId youraccount_to_auction.testnet
    /// ```
    ///
    // NOTE: Currenly only possible if this escrow account has a public key with full access to account, otherwise deploy is not possible.
    #[payable]
    pub fn register(&mut self, title: ValidAccountId) -> Promise {
        let underwriter = ValidAccountId::try_from(env::predecessor_account_id()).unwrap();
        // Make sure this account isnt already in escrow
        assert_ne!(self.accounts.contains_key(&title.to_string()), true, "Account already in escrow");
        log!("Now im trying to delete account");

        ext_transfer_owner::delete_self(
                &title.to_string(),
                0,
                DELETE_ACCOUNT_GAS_FEE
            )
            .then(
                ext_self::on_register_p1(
                    title,
                    underwriter,
                    &env::current_account_id(),
                    env::attached_deposit(),
                    REGISTER_CALLBACK_GAS_FEE
                )
            )
    }

    // TODO: REMOVE
    /// Callback of the register function
    /// Deploys a contract to the new account, must be initialed!
    pub fn p2(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise { //OrValue<bool>
        // if is_promise_success() {
            log!("Now im trying to deploy deed");
            // now that account has been deleted, create again!
            // PromiseOrValue::Promise(
                // deploy deed contract, to allow final management
                Promise::new(title.clone().to_string())
                    .deploy_contract(
                        include_bytes!("../../res/deed.wasm").to_vec(),
                    )
                    // .then(
                    //     ext_self::on_register_p3(
                    //         title,
                    //         underwriter,
                    //         &env::current_account_id(),
                    //         0,
                    //         REGISTER_CALLBACK_GAS_FEE 
                    //     )
                    // )
        //     )
        // } else {
        //     // If register failed, dont do anything we have to revert
        //     PromiseOrValue::Value(false)
        // }
    }

    pub fn p2a(&mut self, title: ValidAccountId) -> Promise { //OrValue<bool>
        // if is_promise_success() {
            log!("Now im trying to add access key");
            // now that account has been deleted, create again!
            // PromiseOrValue::Promise(
                // deploy deed contract, to allow final management
                Promise::new(title.clone().to_string())
                    .add_full_access_key(self.pk.clone())
                    .then(
                        ext_self::p2(
                            title,
                            ValidAccountId::try_from(env::predecessor_account_id()).unwrap(),
                            &env::current_account_id(),
                            0,
                            REGISTER_CALLBACK_GAS_FEE
                        )
                    )
        //     )
        // } else {
        //     // If register failed, dont do anything we have to revert
        //     PromiseOrValue::Value(false)
        // }
    }

    // TODO: REMOVE
    /// Callback of the register function
    /// Responsible for creating the account again, bonding to escrow ownership
    #[payable]
    pub fn p1(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise { //OrValue<bool>  underwriter: ValidAccountId
        // if is_promise_success() {
            log!("Now im trying to create account");

            // now that account has been deleted, create again!
            // PromiseOrValue::Promise(
                ext_account_factory::create_account(
                    title.clone().to_string(),
                    // Base58PublicKey::try_from(env::signer_account_pk()).unwrap(),
                    Base58PublicKey::try_from(self.pk.clone()).unwrap(),
                    &self.factory_id,
                    env::attached_deposit(),
                    NEW_ACCOUNT_GAS_FEE
                )
                .then(
                    // ext_self::p2(
                    //     title,
                    //     ValidAccountId::try_from(env::predecessor_account_id()).unwrap(),
                    //     &env::current_account_id(),
                    //     0,
                    //     REGISTER_CALLBACK_GAS_FEE
                    // )
                    ext_self::p2a(
                        title,
                        &env::current_account_id(),
                        0,
                        REGISTER_CALLBACK_GAS_FEE
                    )
                )
        //     )
        // } else {
        //     // If register failed, dont do anything we have to revert
        //     PromiseOrValue::Value(false)
        // }
    }

    // TODO: Finish gas needs
    /// Allows an owner to cancel a deed, given appropriate parameters
    ///
    /// ```bash
    /// near call _escrow_account_ revert_title '{"title": "some_account.testnet"}' --accountId youraccount.testnet
    /// ```
    pub fn revert_title(&mut self, title: ValidAccountId) -> Promise {
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

    // TODO: Finish gas needs
    /// The full realization of an escrow deed, where the account is
    /// transferred to the new owner
    ///
    /// near call _escrow_account_ close_escrow '{"title": "some_account.testnet", "new_key": "ed25591:PK_HERE"}' --accountId youraccount.testnet
    /// ```
    pub fn close_escrow(&mut self, title: ValidAccountId, new_key: PublicKey) -> Promise {
        self.is_in_escrow(title.clone());

        // Can only be called by auction house
        assert_eq!(self.auction_id, env::predecessor_account_id(), "Must be called only by auction");

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

    /// Gets the data payload of a single task by hash
    ///
    /// ```bash
    /// near view _escrow_account_ in_escrow '{"title": "some_account.testnet"}'
    /// ```
    pub fn in_escrow(&self, title: ValidAccountId) -> bool {
        self.accounts.get(&title.to_string()).is_some()
    }

    /// change the contract basic parameters, in case of needing to upgrade
    /// or change to different account IDs later.
    /// Can only be called by the auction contract
    ///
    /// ```bash
    /// near call _escrow_account_ update_escrow_settings '{"auction_id": "auction2.testnet"}' --accountId auction1.testnet
    /// ```
    pub fn update_escrow_settings(&mut self, auction_id: ValidAccountId) {
        assert_eq!(self.auction_id, env::predecessor_account_id(), "Callee must be auction contract");
        self.auction_id = auction_id.to_string();
    }

    fn is_in_escrow(&self, title: ValidAccountId) {
        assert_eq!(self.accounts.contains_key(&title.to_string()), true, "Account not in escrow");
    }

    fn is_underwriter(&self, title: ValidAccountId) {
        assert_eq!(self.accounts.get(&title.to_string()).unwrap(), env::predecessor_account_id(), "Account cannot control escrow account");
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
    // auction: Acct 1
    // escrow (me): Acct 2
    fn create_blank_escrow() -> Escrow {
        Escrow::new(
            accounts(0),
            accounts(1),
            Base58PublicKey::try_from("ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_string()).unwrap()
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
        assert_eq!(contract.factory_id, accounts(0).to_string());
        assert_eq!(contract.auction_id, accounts(1).to_string());
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

        contract.register(accounts(2));
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

        contract.register(accounts(2));

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
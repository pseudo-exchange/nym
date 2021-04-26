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
const NEW_ACCOUNT_STORAGE_AMOUNT: u128 = ONE_NEAR * 3;
const MAX_GAS_FEE: u64 = 300_000_000_000_000;
const NEW_ACCOUNT_GAS_FEE: u64 = MAX_GAS_FEE / 3;
const DELETE_ACCOUNT_GAS_FEE: u64 = MAX_GAS_FEE / 3;
const REGISTER_CALLBACK_GAS_FEE: u64 = MAX_GAS_FEE / 2;
// const REGISTRAR: &str = "registrar";

#[ext_contract(ext_self)]
trait ExtSelf {
    fn finalize_register(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise;
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
    fn new(&mut self, escrow_account_id: ValidAccountId);
    fn revert_ownership(&mut self) -> Promise;
    fn remove_key(&mut self, remove_key: PublicKey) -> Promise;
    fn transfer_ownership(&mut self, new_owner_pk: PublicKey) -> Promise;
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

    // TODO: Owner -- auction house?
    owner_id: Option<AccountId>,

    // keeps track of the escrowed accounts
    tlas: LookupMap<AccountId, AccountId>,
    accounts: LookupMap<AccountId, AccountId>
}

/// Escrow
/// Contract keeps track of accounts in escrow
/// is the only account that can execute functions on escrowed account
/// should not be owned by anyone -- NO ACCESS KEYS!
#[near_bindgen]
impl Escrow {
    #[init(ignore_state)]
    pub fn new(
        factory_id: AccountId,
        auction_id: AccountId,
        pk: PublicKey
    ) -> Self {
        Escrow {
            id: env::current_account_id(),
            pk,
            factory_id,
            auction_id,
            tlas: LookupMap::new(ESCROW_STORAGE_TLA_KEY.to_vec()),
            accounts: LookupMap::new(ESCROW_STORAGE_ACCOUNT_KEY.to_vec()),
            owner_id: None,
        }
    }

    // TODO: finish
    /// Callback of the register function
    /// Responsible for creating the account again, bonding to escrow holdings
    /// Deploys a contract to the new account, such that
    pub fn finalize_register(&mut self, title: ValidAccountId, underwriter: ValidAccountId) -> Promise {
        // add to registry
        self.accounts.insert(&title.to_string(), &underwriter.to_string());
        log!("New deed: {}", &title);

        // now that account has been deleted, create again!
        ext_account_factory::create_account(
            title.clone().to_string(),
            Base58PublicKey::try_from(env::signer_account_pk()).unwrap(),
            &self.factory_id,
            NEW_ACCOUNT_STORAGE_AMOUNT,
            NEW_ACCOUNT_GAS_FEE
        )
        .then(
            // deploy deed contract, to allow final management
            Promise::new(underwriter.clone().to_string())
                .deploy_contract(
                    include_bytes!("../../res/deed.wasm").to_vec(),
                )
                // TODO: 
                // .then(ext_deed::new())
        )
    }

    // TODO:
    /// Responsible for bonding an account to a deed contract, where
    /// escrow is the sole owner, and can only transfer ownership upon
    /// close of title
    // NOTE: Currenly only possible if this escrow account has a public key with full access to account, otherwise deploy is not possible.
    #[payable]
    pub fn register(&mut self, underwriter: ValidAccountId) -> Promise {
        let title = ValidAccountId::try_from(env::signer_account_id()).unwrap();
        // Make sure this account isnt already in escrow
        assert_ne!(self.accounts.contains_key(&title.to_string()), true, "Account already in escrow");

        Promise::new(title.clone().to_string())
            .function_call(
                b"delete_self".to_vec(),
                json!({}).to_string().as_bytes().to_vec(),
                0,
                DELETE_ACCOUNT_GAS_FEE
            )
            .then(
                ext_self::finalize_register(
                    title,
                    underwriter,
                    &env::current_account_id(),
                    0,
                    REGISTER_CALLBACK_GAS_FEE
                )
            )
    }

    // TODO: Finish gas needs
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

    // TODO: Finish gas needs
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

    // TODO: Finish gas needs
    /// The full realization of an escrow deed, where the account is
    /// transferred to the new owner
    pub fn close_escrow(&mut self, title: AccountId, new_key: PublicKey) -> Promise {
        self.is_in_escrow(title.clone());
        // TODO: 
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
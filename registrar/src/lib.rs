use near_sdk::{
    near_bindgen,
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{ UnorderedMap, TreeMap},
    json_types::{ ValidAccountId, Base58PublicKey },
    serde_json::json,
    serde::{Deserialize, Serialize},
    AccountId,
    Balance,
    BlockHeight,
    PanicOnDefault,
    Promise,
    PublicKey,
    env,
    log,
    BorshStorageKey,
    StorageUsage,
};
use bs58;

near_sdk::setup_alloc!();

pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
const AUCTION_STORAGE_COST: u128 = 2_000_000_000_000_000_000_000;
// const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000;
const CHECK_UNDERWRITER_GAS_FEE: u64 = 5_000_000_000_000; // 5 Tgas
const CREATE_CALLBACK_GAS_FEE: u64 = 150_000_000_000_000; // 150 Tgas
const CLOSE_BLOCK_OFFSET: u64 = 600_000; // ~7 days
const REVEAL_BLOCK_OFFSET: u64 = 260_000; // ~3 days

// TODO: Cron fee & schedule setup

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Auctions,
    Bids,
    Reveals,
}

#[ext_contract(ext)]
pub trait ExtRegistrar {
    fn create_callback(
        &mut self,
        title: ValidAccountId,
        signer: AccountId,
        auction_start_bid_amount: Balance,
        auction_close_block: Option<BlockHeight>,
        is_blind: Option<bool>,
        #[callback]
        underwriter: Option<AccountId>,
    );
}

#[ext_contract(ext_escrow)]
pub trait ExtEscrow {
    fn get_underwriter(&self, title: ValidAccountId) -> Option<AccountId>;
    fn close_escrow(&mut self, title: AccountId, new_key: PublicKey) -> Promise;
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Bid {
    amount: Balance,
    pk: PublicKey,
    precommit: Option<Vec<u8>>
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
// #[serde(crate = "near_sdk::serde")]
pub struct Auction {
    pub title: AccountId,
    pub is_blind: bool,
    pub underwriter: Option<AccountId>,
    pub winner_id: Option<AccountId>,
    pub close_block: Option<BlockHeight>,
    bids: UnorderedMap<AccountId, Bid>,
    reveals: TreeMap<Balance, AccountId>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Registrar {
    pub paused: bool,
    pub base_fee: Balance,
    pub base_storage_usage: StorageUsage,
    pub auctions: UnorderedMap<AccountId, Auction>,

    // Admin only
    pub escrow: AccountId,
    pub dao: Option<AccountId>,
}

// TODO: Add admin FNs for pause/unpause
#[near_bindgen]
impl Registrar {
    /// Constructor:
    /// See notes regarding escrow contract, ownership & state separation
    /// This method instantiates new registrar contract with baseline config
    /// 
    /// ```bash
    /// near deploy --wasmFile res/registrar.wasm --initFunction new --initArgs '{"escrow_account_id": "escrow_account.testnet", "escrow_pk": "ed25591:jfsdofa..."}' --accountId registrar_account.testnet
    /// ```
    #[init]
    pub fn new(escrow: ValidAccountId, dao: Option<ValidAccountId>) -> Self {
        // Make absolutely sure this contract doesnt get state removed easily
        // TODO: Change to support migrations
        assert!(!env::state_exists(), "The contract is already initialized");
        assert_eq!(env::current_account_id(), env::predecessor_account_id(), "Must be called by owner");

        let mut this = Registrar {
            paused: false,
            base_fee: ONE_NEAR / 100_000,
            base_storage_usage: 0,
            auctions: UnorderedMap::new(StorageKeys::Auctions),
            escrow: escrow.to_string(),
            dao: Some(dao.unwrap().to_string()),
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
        let tmp_auction = Auction {
            title: tmp_account_id,
            is_blind: true,
            underwriter: Some(tmp_account_id),
            winner_id: Some(tmp_account_id),
            close_block: Some(env::block_index()),
            bids: UnorderedMap::new(b"a".to_vec()),
            reveals: TreeMap::new(b"b"),
        };
        self.auctions.insert(&tmp_account_id, &tmp_auction);
        self.base_storage_usage = env::storage_usage() - initial_storage_usage;
        // Remove the temporary entry.
        self.auctions.remove(&tmp_account_id);
    }

    /// Create Auction
    /// Allows an underwriter to create a new auction for an account they own.
    /// The underwriter is the original owner or another account that takes ownership in the event
    /// the auction closes with no winner or underwriter wants to claim the account back before auction close.
    /// 
    /// Defaults:
    /// auction_close_block: 7 Days
    /// auction reveals: 3 Days
    ///
    /// ```bash
    /// near call _auction_ create '{"title": "account_to_auction.testnet", "auction_start_bid_amount": 1, "auction_close_block": 41000000, "is_blind": true}' --accountId youraccount.testnet
    /// ```
    #[payable]
    pub fn create(
        &mut self,
        title: ValidAccountId,
        auction_start_bid_amount: Balance,
        auction_close_block: Option<BlockHeight>,
        is_blind: Option<bool>
    ) {
        // Check if there is already an auction with this same matching title
        // AND if that auction is ongoing (ongoing = current block < closing block)
        let previous_auction = self.auctions.get(&title.to_string());
        if previous_auction.is_some() {
            assert!(
                env::block_index() > previous_auction.unwrap().close_block.unwrap(),
                "Auction is already happening"
            );
        }

        // Confirm escrow has custody
        ext_escrow::get_underwriter(
            title,
            &self.escrow,
            0,
            CHECK_UNDERWRITER_GAS_FEE
        ).then(
            ext::create_callback(
                title,
                env::signer_account_id(),
                auction_start_bid_amount,
                auction_close_block,
                is_blind,
                &env::current_account_id(),
                env::attached_deposit(),
                CREATE_CALLBACK_GAS_FEE,
            )
        );
    }

    /// Create Auction Callback
    #[private]
    #[payable]
    pub fn create_callback(
        &mut self,
        title: ValidAccountId,
        signer: AccountId,
        auction_start_bid_amount: Balance,
        auction_close_block: Option<BlockHeight>,
        is_blind: Option<bool>,
        #[callback]
        underwriter: Option<AccountId>,
    ) {
        // Check the signer IS the underwriter
        let owner = underwriter.expect("No underwriter found, abort");
        assert_eq!(&signer, &owner, "Auction can only be started by owner");

        let close_block = match auction_close_block {
            Some(close_block) => {
                if close_block > env::block_index() { close_block } else { env::block_index() + CLOSE_BLOCK_OFFSET }
            },
            None => env::block_index() + CLOSE_BLOCK_OFFSET,
        };

        let auction = Auction {
            title: title.to_string(),
            is_blind: is_blind.unwrap_or(false),
            underwriter: Some(owner),
            winner_id: None,
            close_block: Some(close_block),
            bids: UnorderedMap::new(StorageKeys::Bids),
            reveals: TreeMap::new(StorageKeys::Reveals)
        };

        self.auctions.insert(&title.to_string(), &auction);
        log!("New Auction:{}", &title.to_string());
    }

    // return single auction item
    ///
    /// ```bash
    /// near view _auction_ get_auction_byid '{"id": "account_to_auction.testnet"}'
    /// ```
    pub fn get_auction_byid(&self, id: AccountId) -> Option<Auction> {
        self.auctions.get(&id)
    }

    // return single auction item
    pub fn get_auction_by_id(&self, id: AccountId) -> String {
        let auction = self.auctions.get(&id).expect("No auction found");

        json!({
            "underwriter": auction.underwriter,
            "winner_id": auction.winner_id.unwrap(),
            "title": auction.title,
            "close_block": auction.close_block,
            // TODO: Stringify this
            "bids": auction.bids.len(),
            "reveals": auction.reveals.len(),
            // "bids": json!(auction.bids).to_string(),
            // "reveals": auction.reveals.to_string()
        }).to_string()
    }

    /// Bid:
    /// Allow anyone to place a bid on an auction,
    /// which accepts an auction id and attached_deposit balance for contribution which buys the asset
    ///
    /// Requires:
    /// - user to NOT be owner
    /// - bid amount needs to be greater than 0
    /// - auction needs to not be closed
    ///
    /// Optional:
    /// - amount: if no deposit, then MUST be blind bid
    /// - updates: user CAN update bid by calling this fn multiple times
    ///
    /// Blind auctions require a commit/reveal setup. In this way, we can create a time boundary to give
    /// auctions a more fair price outcome. Winner is still the highest bid, but with reveal phase outside
    /// the normal bid phase, we can guarantee frontrunning doesnt skew price to some extent.
    /// Commit in this context is just a number + salt string that is hashed.
    ///
    /// ```bash
    /// near call _auction_ bid '{"id": "auctioned_account.testnet", "pk": "ed25519:abcd...", "commit": [100,50,10...]}' --accountId youraccount.testnet --amount 13
    /// ```
    #[payable]
    pub fn bid(
        &mut self,
        id: AccountId,
        pk: Base58PublicKey,
        commit: Option<Vec<u8>>
    ) {
        let auc = self.auctions.get(&id).expect("Auction doesnt exist");
        assert_ne!(
            auc.underwriter.unwrap(),
            env::signer_account_id(),
            "Must not be owner of auction"
        );
        assert!(
            env::block_index() < auc.close_block.unwrap(),
            "Must be an active auction"
        );

        let mut auction = self.auctions.get(&id).expect("Auction doesnt exist");
        let is_blind = auction.is_blind;

        // Check if auction requires blind auction
        // Otherwise, make sure the bid has a deposit
        if is_blind && commit.is_none() {
            panic!("Auction requires blind bid");
        } else {
            assert!(
                env::attached_deposit() > 0,
                "Must submit bid amount of greater than zero"
            );
        }

        // Accept Deposit as bid amount
        // Keep track of how much balance user sent
        let bid = Bid {
            amount: Some(env::attached_deposit()).unwrap_or(0),
            pk: pk.clone().into(),
            precommit: Some(commit).unwrap_or(None),
        };
        
        // Update storage
        auction.bids.insert(&env::signer_account_id(), &bid);
        self.auctions.insert(&id, &auction);
    }

    /// Reveal: Optional -- used for Blind Auctions
    /// Reveal allows the user to unmask their bid amount, and actually pay what they said that would.
    /// Because the masked amount needs to actually be paid, we expect the sent deposit to match
    ///
    /// ```bash
    /// near call _auction_ reveal '{"id": "auctioned_account.testnet", "salt": "super_secret"}' --accountId youraccount.testnet --amount 1337
    /// ```
    #[payable]
    pub fn reveal(&mut self, id: ValidAccountId, salt: String) {
        let auc = self.auctions.get(&id.to_string()).expect("Auction doesnt exist");
        assert_ne!(
            auc.underwriter.unwrap(),
            env::signer_account_id(),
            "Must not be owner of auction"
        );
        assert!(
            env::block_index() > auc.close_block.unwrap()
            && env::block_index() < auc.close_block.unwrap() + REVEAL_BLOCK_OFFSET,
            "Must be reveal phase in auction"
        );

        let mut auction = self.auctions.get(&id.to_string()).expect("Auction doesnt exist");
        let is_blind = auction.is_blind;

        // auction requires reveal bid data
        if is_blind && salt.len() < 1 {
            panic!("Auction requires blind bid");
        }

        let deposit: u128 = env::attached_deposit();
        let reveal_str: String = deposit.to_string() + &salt;
        let reveal_hash: Vec<u8> = bs58::encode(&reveal_str).into_string().as_bytes().to_vec();

        // Check that reveal matches precommit
        let bid = auction.bids.get(&env::signer_account_id()).expect("No bid found");
        assert_eq!(bid.precommit.unwrap(), reveal_hash, "Reveal doesnt match original bid");

        // Update storage
        auction.reveals.insert(&deposit, &env::signer_account_id());
        self.auctions.insert(&id.to_string(), &auction);
    }

    /// Cancel Auction:
    /// removes an auction if owner called it
    /// sends back all auction bidders their funds
    ///
    /// ```bash
    /// near call _auction_ cancel_auction '{"id": "auctioned_account.testnet"}' --accountId youraccount.testnet
    /// ```
    pub fn cancel_auction(&mut self, id: String) {
        let auction = self.auctions.get(&id).expect("No auction found");
        assert!(
            env::block_index() < auction.close_block.unwrap(),
            "Auction must not be complete"
        );
        assert_eq!(
            auction.underwriter.unwrap(),
            env::predecessor_account_id(),
            "Must be owner to cancel auction"
        );

        // Loop to return bid funds
        let bids = auction.bids.iter();
        for (account_id, Bid { amount, pk: _, precommit: _ }) in bids {
            if amount > 0 {
                Promise::new(account_id).transfer(amount);
            }
        }

        // Release from escrow
        ext_escrow::close_escrow(
            id.clone(),
            env::signer_account_pk(),
            &self.escrow,
            0,
            CLOSE_ESCROW_GAS_FEE,
        );

        // Clear auction storage, since this is over
        self.auctions.remove(&id);
    }

    /// Finalize Auction:
    /// - award winner the asset, if they were highest bidder
    /// - all bidders get their bid amounts back, minus fees
    ///
    /// NOTE: anyone can call this method, as it is paid by the person wanting the final outcome
    ///
    /// ```bash
    /// near call _auction_ finalize_auction '{"id": "auctioned_account.testnet"}' --accountId youraccount.testnet
    /// ```
    pub fn finalize_auction(&mut self, id: AccountId) {
        // Get auction details
        let auction = self.auctions.get(&id).expect("No auction found");
        assert!(
            env::block_index() > auction.close_block.unwrap(),
            "Auction must be complete"
        );
        log!("Finalize Auction: {}", &id);

        // Find winner, refund others
        let mut winner_id: AccountId = "".to_string();
        let mut winner_pk: PublicKey = vec![0];
        let mut highest_balance: Balance = 0;

        let bids = auction.bids;
        let reveals = auction.reveals;

        if auction.is_blind {
            // Since reveals is treemap, just sort by highest bid amount (key)
            let winning_key = reveals.max().expect("No reveals found");
            let winning_account_id = reveals.get(&winning_key).expect("No reveal account found");
            let winning_bid = bids.get(&winning_account_id).expect("No bid found for reveal");
            winner_id = winning_account_id;
            winner_pk = winning_bid.pk;
        } else {
            // Loop to find winner
            for (account_id, Bid { amount, pk, precommit: _}) in bids.iter() {
                if highest_balance < amount {
                    highest_balance = amount;
                    winner_id = account_id;
                    winner_pk = pk;
                }
            }
        }

        // Loop to return losing funds, minus fees
        for (account_id, Bid { amount, pk: _, precommit: _ }) in bids.iter() {
            if winner_id != account_id && amount > self.base_fee {
                Promise::new(account_id).transfer(amount - self.base_fee);
            }
        }

        // Release from escrow
        ext_escrow::close_escrow(
            id.clone(),
            winner_pk,
            &self.escrow_account_id.clone().unwrap(),
            0,
            // TODO: Change to better value
            env::prepaid_gas() / 3
        );

        // Clear auction storage, since this is over
        self.auctions.remove(&id);
    }

    /// Hash:
    /// Tiny helper method to calculate a base58 hash of an amount + salt
    /// NOTE: using the command below should only be used for testing, network requests reveal real information to RPC runners.
    ///
    /// ```bash
    /// near view _auction_ hash '{"amount": 10, "salt": "super_secret"}'
    /// ```
    pub fn hash(&self, amount: Balance, salt: String) ->  Vec<u8> {
        bs58::encode(amount.to_string() + &salt).into_string().as_bytes().to_vec()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};
    use std::convert::TryFrom;

    // registrar (me): Acct 0
    // auction: Acct 1
    // escrow: Acct 2
    fn create_blank_registrar() -> Registrar {
        Registrar::new(
            ValidAccountId::try_from("escrow_near").unwrap(),
            Base58PublicKey::try_from("ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6").unwrap(),
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
        // Init with escrow data
        let contract = create_blank_registrar();

        assert_eq!(
            false, contract.paused,
            "Auction MUST not be paused initially"
        );

        assert_eq!(
            "escrow_near".to_string(),
            contract.escrow_account_id.unwrap(),
            "Escrow account ID is set appropriately"
        );

        assert_eq!(
            b"ed25519:AtysLvy7KGoE8pznUgXvSHa4vYyGvrDZFcT8jgb8PEQ6".to_vec(),
            contract.escrow_pk.unwrap(),
            "Escrow account public key is set appropriately"
        );

        // TODO: Figure out how to test this!
        // assert_eq!(
        //     env::signer_account_pk(),
        //     // HOw do i get contract full access keys list?,
        //     "Ensure the contract is owned by deployment signer"
        // );
    }

    #[test]
    #[should_panic(expected = "Auction is already happening")]
    fn new_auction_item_same_during_auction() {
        let mut context = get_context(accounts(3), accounts(3), accounts(3), Some(true));
        testing_env!(context.build());
        // Init with escrow data
        let mut contract = create_blank_registrar();
        // ----------------------------------------------------------------
        // THIS IS HOW THE BLOCKCHAIN PROGRESSES STATE
        // IF YOU ARE USING ANY TYPE OF PROMISE OR NON-VIEW FN,
        // YOU MUST CHANGE "is_view" TO SHOW THE TEST RUNNER TO DO THE SHITS
        // ----------------------------------------------------------------
        context.is_view(false);
        testing_env!(context.build());

        // call the contract create twice, so we can panic when the auction item already exists
        // AND is active (within the current block height)
        contract.create(
            ValidAccountId::try_from("zanzibar_near").unwrap(),
            ValidAccountId::try_from("yokohama_near").unwrap(),
            1 * ONE_NEAR,
            Some(1_000),
            Some(false)
        );
        testing_env!(context.build());
        contract.create(
            ValidAccountId::try_from("zanzibar_near").unwrap(),
            ValidAccountId::try_from("yokohama_near").unwrap(),
            1 * ONE_NEAR,
            Some(1_000),
            Some(false)
        );
    }

    #[test]
    #[should_panic(expected = "Auction cannot be signer name")]
    fn new_auction_item_not_same_as_signer() {
        let context = get_context(accounts(3), accounts(3), accounts(3), Some(false));
        testing_env!(context.build());
        // Init with escrow data
        let mut contract = create_blank_registrar();

        // call the contract create twice, so we can panic when the auction item already exists
        // AND is active (within the current block height)
        contract.create(
            ValidAccountId::try_from("bob_near").unwrap(),
            ValidAccountId::try_from(env::signer_account_id()).unwrap(),
            1 * ONE_NEAR,
            Some(env::block_index() + 1_000),
            Some(false)
        );
    }

    #[test]
    fn create_auction_item() {
        let context = get_context(accounts(3), accounts(3), accounts(3), Some(false));
        testing_env!(context.build());
        // Init with escrow data
        let mut contract = create_blank_registrar();

        // check all the auction item THANGS
        contract.create(
            ValidAccountId::try_from("zanzibar_near").unwrap(),
            ValidAccountId::try_from(env::signer_account_id()).unwrap(),
            1 * ONE_NEAR,
            Some(env::block_index() + 1_000),
            Some(false)
        );

        assert_eq!(
            1,
            contract.auctions.len(),
            "Contract: Creates new auction item"
        );

        // assert!("Contract: Adds Auction House as full access key");

        // assert!("Contract: Removes all other access keys");

        // assert!("Contract: Returns newly created auction item ID");
    }
}

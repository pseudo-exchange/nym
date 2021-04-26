use near_sdk::{
    near_bindgen,
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{ UnorderedMap, TreeMap},
    json_types::{ ValidAccountId, Base58PublicKey },
    serde_json::json,
    AccountId,
    Balance,
    BlockHeight,
    PanicOnDefault,
    Promise,
    PublicKey,
    env,
    log
};
use bs58;

near_sdk::setup_alloc!();

pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
// const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000;
const CLOSE_BLOCK_OFFSET: u64 = 1_000_000;

// fn only_admin() {
//     // require only admins
//     assert_eq!(
//         &env::current_account_id(),
//         &env::signer_account_id(),
//         "Only owner can execute this fn",
//     )
// }

#[ext_contract]
pub trait ExtEscrow {
    fn register(&mut self, underwriter: ValidAccountId);
    fn in_escrow(&self, title: ValidAccountId) -> bool;
    fn revert_title(&mut self, title: AccountId) -> Promise;
    fn close_escrow(&mut self, title: AccountId, new_key: PublicKey) -> Promise;
    fn update_escrow_settings(&mut self, auction_id: ValidAccountId);
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Bid {
    amount: Balance,
    pk: PublicKey,
    precommit: Option<Vec<u8>>
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
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
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AuctionHouse {
    pub auctions: UnorderedMap<AccountId, Auction>,
    pub paused: bool,
    pub escrow_account_id: Option<AccountId>,
    pub escrow_pk: Option<PublicKey>,
    pub base_fee: Balance,
}

impl Default for AuctionHouse {
    fn default() -> Self {
        AuctionHouse {
            paused: false,
            escrow_account_id: None,
            escrow_pk: None,
            auctions: UnorderedMap::new(b"a".to_vec()),
            base_fee: ONE_NEAR / 100_000,
        }
    }
}

// TODO: Add admin FNs for pause/unpause
#[near_bindgen]
impl AuctionHouse {
    /// Constructor:
    /// See notes regarding escrow contract, ownership & state  separation
    /// This method instantiates new auction house contract with baseline config
    ///
    /// ```bash
    /// near call _auction_ new '{"escrow_account_id": "escrow_account.testnet", "escrow_pk": "ed25591:jfsdofa..."}' --accountId youraccount.testnet
    /// ```
    #[init]
    pub fn new(escrow_account_id: ValidAccountId, escrow_pk: Base58PublicKey) -> Self {
        // Make absolutely sure this contract doesnt get state removed easily
        assert!(!env::state_exists(), "The contract is already initialized");

        AuctionHouse {
            paused: false,
            auctions: UnorderedMap::new(b"a".to_vec()),
            escrow_account_id: Some(escrow_account_id.to_string()),
            escrow_pk: Some(escrow_pk.into()),
            base_fee: ONE_NEAR / 100_000,
        }
    }

    // TODO: Confirm an asset is not being auctioned again during an active auction
    // TODO: Check if this call originated from escrow?
    // TODO: Create optional blind auction setup
    // TODO: Get fee
    #[payable]
    #[allow(unused_variables)] // TODO: remove when impl done
    pub fn create(
        &mut self,
        title: ValidAccountId,
        underwriter: ValidAccountId,
        auction_start_bid_amount: Balance,
        auction_close_block: Option<BlockHeight>,
        is_blind: Option<bool>
    ) {
        assert_eq!(
            &title.to_string(),
            &env::signer_account_id(),
            "Auction must be signer name"
        );

        let close_block = match auction_close_block {
            Some(close_block) => close_block,
            None => env::block_index() + CLOSE_BLOCK_OFFSET,
        };

        let auction = Auction {
            title: title.to_string(),
            is_blind: is_blind.unwrap_or(false),
            underwriter: Some(underwriter.to_string()),
            winner_id: None,
            close_block: Some(close_block),
            bids: UnorderedMap::new(b"a".to_vec()),
            reveals: TreeMap::new(b"b")
        };

        // Check if there is already an auction with this same matching title
        // AND if that auction is ongoing (ongoing = current block < closing block)
        let previous_auction = self.auctions.get(&title.to_string());
        match previous_auction {
            Some(previous_auction) => {
                assert!(
                    env::block_index() > previous_auction.close_block.unwrap(),
                    "Auction is already happening"
                );
            }
            None => (),
        }

        self.auctions.insert(&title.to_string(), &auction);
        log!("New Auction:{}", &title.to_string());

        // TODO: Confirm escrow has custody
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
            "reveals": auction.reveals.len()
        }).to_string()
    }

    // Allow anyone to place a bid on an auction,
    // which accepts an auction id and attached_deposit balance for contribution which buys the asset
    //
    // Requires:
    // - user to NOT be owner
    // - auction amount needs to be greater than 0
    // - auction needs to not be closed
    //
    // Optional:
    // - user CAN update bid by calling this fn multiple times
    #[payable]
    pub fn bid(
        &mut self,
        id: AccountId,
        _pk: Base58PublicKey
    ) -> Promise {
        match self.auctions.get(&id) {
            Some(auction) => {
                assert_ne!(
                    auction.underwriter,
                    env::signer_account_id(),
                    "Must not be owner of auction"
                );
                assert!(
                    env::attached_deposit() > 0,
                    "Must submit bid amount of greater than zero"
                );
                assert!(
                    env::block_index() < auction.close_block.unwrap(),
                    "Must be an active auction"
                );
            }
            None => {
                panic!("Bid does not meet parameters");
            }
        }

        // TODO: Finish
        // Accept Balance as bid amount
        // Keep track of how much balance user sent
        Promise::new(self.escrow_account_id.as_ref().unwrap().clone())
            .transfer(env::attached_deposit())
    }

    // NOTE: For TLAs, auctions can only be done against "registrar"
    // Auctions for TLAs also must go through strict string schedules before starting bids.
    // Restriction schedules consist of:
    // 1. time lock
    // 2. char length
    // 3. char uniqueness
    // 4. sha hash modulo 52 (debatable)
    // 5. min bid size, based on char length

    // NOTE: Registrar currently is public_key signed to create new accounts
    // For auctions to work, either things should happen:
    // 1. deploy a contract to registrar that accepts name creation, based on params
    // 2. change the protocol to enable TLAs action for certain contracts (core contract)

    // TODO:
    /// Blind auctions require a commit/reveal setup. In this way, we can create a time boundary to give
    /// auctions a more fair price outcome. Winner is still the highest bid, but with reveal phase outside
    /// the normal bid phase, we can guarantee frontrunning doesnt skew price to some extent.
    /// Commit in this context is just a number + salt string that is hashed.
    pub fn bid_blind(&mut self, id: ValidAccountId, commit: Vec<u8>) {

    }

    /// Reveal allows the user to unmask their bid amount, and actually pay what they said that would.
    /// Because the masked amount needs to actually be paid, we expect the sent deposit to match
    #[payable]
    pub fn reveal(&mut self, id: ValidAccountId, salt: String) {
        let reveal_str: String = env::attached_deposit().to_string() + &salt;
        let reveal_hash: Vec<u8> = bs58::encode(&reveal_str).into_string().as_bytes().to_vec();
    }

    // removes an auction if owner called it
    // sends back all auction bidders their funds
    pub fn cancel_auction(&mut self, id: String) {
        let auction = self.auctions.get(&id).expect("No auction found");
        assert!(
            env::block_index() < auction.close_block.unwrap(),
            "Auction must not be complete"
        );
        assert_eq!(
            auction.underwriter,
            env::signer_account_id(),
            "Must be owner to cancel auction"
        );

        // Loop to return losing funds, minus fees
        let bids = auction.bids.iter();
        for (account_id, Bid { amount, pk: _, precommit: _ }) in bids {
            if amount > 0 {
                Promise::new(account_id).transfer(amount);
            }
        }

        // Release from escrow
        ext_escrow::revert_title(
            id.clone(),
            &self.escrow_account_id.clone().unwrap(),
            0,
            // TODO: Change to better value
            env::prepaid_gas() / 3
        );

        // Clear auction storage, since this is over
        self.auctions.remove(&id);
    }

    // finalize auction:
    // - award winner the asset, if they were highest bidder
    // - all bidders get their bid amounts back, minus fees
    //
    // NOTE: anyone can call this method, as it is paid by the person wanting the final outcome
    pub fn finalize_auction(&mut self, id: AccountId) {
        // Get auction details
        let auction = self.auctions.get(&id).expect("No auction found");
        assert!(
            env::block_index() > auction.close_block.unwrap(),
            "Auction must be complete"
        );
        log!("Finalize Auction:{}", &id);

        // Find winner, refund others
        let mut winner_id: AccountId = "".to_string();
        let mut winner_pk: PublicKey = vec![0];
        let mut highest_balance: Balance = 0;

        let bids = auction.bids;
        
        // Loop to find winner
        for (account_id, Bid { amount, pk, precommit: _}) in bids.iter() {
            if highest_balance < amount {
                highest_balance = amount;
                winner_id = account_id;
                winner_pk = pk;
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
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn create_blank_auction_house() -> AuctionHouse {
        AuctionHouse::new(
            "escrow_near".to_string(),
            Base58PublicKey { 0: vec![0, 1, 2] },
        )
    }

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    #[test]
    fn initialize_constructor() {
        let context = get_context(vec![], true);
        testing_env!(context);
        // Init with escrow data
        let contract = create_blank_auction_house();

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
            Base58PublicKey { 0: vec![0, 1, 2] },
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
        let mut context = get_context(vec![], true);
        testing_env!(context.clone());
        // Init with escrow data
        let mut contract = create_blank_auction_house();
        // ----------------------------------------------------------------
        // THIS IS HOW THE BLOCKCHAIN PROGRESSES STATE
        // IF YOU ARE USING ANY TYPE OF PROMISE OR NON-VIEW FN,
        // YOU MUST CHANGE "is_view" TO SHOW THE TEST RUNNER TO DO THE SHITS
        // ----------------------------------------------------------------
        context.is_view = false;
        testing_env!(context.clone());

        // call the contract create twice, so we can panic when the auction item already exists
        // AND is active (within the current block height)
        contract.create(
            "zanzibar_near".to_string(),
            "yokohama_near".to_string(),
            Some(1_000),
            1 * ONE_NEAR,
        );
        testing_env!(context.clone());
        contract.create(
            "zanzibar_near".to_string(),
            "yokohama_near".to_string(),
            Some(1_000),
            1 * ONE_NEAR,
        );
    }

    #[test]
    #[should_panic(expected = "Auction cannot be signer name")]
    fn new_auction_item_not_same_as_signer() {
        let context = get_context(vec![], true);
        testing_env!(context);
        // Init with escrow data
        let mut contract = create_blank_auction_house();

        // call the contract create twice, so we can panic when the auction item already exists
        // AND is active (within the current block height)
        contract.create(
            "bob_near".to_string(),
            env::signer_account_id(),
            Some(env::block_index() + 1_000),
            1 * ONE_NEAR,
        );
    }

    #[test]
    fn create_auction_item() {
        let context = get_context(vec![], true);
        testing_env!(context);
        // Init with escrow data
        let mut contract = create_blank_auction_house();

        // check all the auction item THANGS
        contract.create(
            "zanzibar_near".to_string(),
            env::signer_account_id(),
            Some(env::block_index() + 1_000),
            1 * ONE_NEAR,
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

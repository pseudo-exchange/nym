use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::Base58PublicKey;
use near_sdk::{env, near_bindgen, AccountId, Balance, BlockHeight, Promise};
mod util;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000;
const CLOSE_BLOCK_OFFSET: u64 = 1_000_000;

// fn only_admin() {
//     // require only admins
//     assert_eq!(
//         &env::current_account_id(),
//         &env::signer_account_id(),
//         "Only owner can execute this fn",
//     )
// }

// TODO: How to i get the current list of access keys?
// TODO: One way to do this (BADLY) is to delete the account entirely,
// cause this contract to be beneficiary,
// then recreate it and send overflow balance to previous owner -- this is not a great solution
fn transfer_ownership(
    from_account_id: AccountId,
    from_public_key: Base58PublicKey,
    to_public_key: Base58PublicKey,
    to_account_id: AccountId,
) -> Promise {
    // TODO: Remove once fully tested
    logger!("from_account_id: {:?}", &from_account_id);
    logger!("from_public_key: {:?}", &from_public_key);
    logger!("to_public_key: {:?}", &to_public_key);
    logger!("to_account_id: {:?}", &to_account_id);

    // Here be the magix
    // First grant all access keys to the escrow account
    Promise::new(to_account_id).add_full_access_key(to_public_key.into());
    // Next remove all other access keys, so only the escrow account "owns" the
    // TODO: Make sure this deletes all PKs -- I dont think i found that yet
    Promise::new(from_account_id).delete_key(from_public_key.into())
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Auction {
    pub owner_id: AccountId, // near account
    pub winner_account_id: Option<AccountId>,
    pub asset: AccountId,
    pub close_block: Option<BlockHeight>, // Needs checking that theres no race case transactions
    bids: UnorderedMap<AccountId, Balance>,
}

impl ToString for Auction {
    fn to_string(&self) -> String {
        let fields = vec![
            self.owner_id.to_string(),
            self.asset.to_string(),
            self.close_block.unwrap().to_string(),
        ];
        fields.join("")
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AuctionHouse {
    pub auctions: UnorderedMap<String, Auction>,
    pub paused: bool,
    pub escrow_account_id: Option<AccountId>,
    pub escrow_public_key: Option<Base58PublicKey>,
}

impl Default for AuctionHouse {
    fn default() -> Self {
        AuctionHouse {
            paused: false,
            escrow_account_id: None,
            escrow_public_key: None,
            auctions: UnorderedMap::new(env::keccak256(env::block_index().to_string().as_bytes())),
        }
    }
}

// TODO: Add admin FNs for pause/unpause
#[near_bindgen]
impl AuctionHouse {
    /// Constructor:
    /// See notes regarding escrow contract, ownership & state  separation
    /// This method instantiates new auction house contract with baseline config
    #[init]
    pub fn new(escrow_account_id: AccountId, escrow_public_key: Base58PublicKey) -> Self {
        // Make absolutely sure this contract doesnt get state removed easily
        assert!(!env::state_exists(), "The contract is already initialized");
        assert!(
            env::is_valid_account_id(&escrow_account_id.as_bytes()),
            "Must be a valid escrow contract id"
        );
        AuctionHouse {
            paused: false,
            auctions: UnorderedMap::new(env::keccak256(env::block_index().to_string().as_bytes())),
            escrow_account_id: Some(escrow_account_id),
            escrow_public_key: Some(escrow_public_key),
        }
    }

    // TODO: Confirm an asset is not being auctioned again during an active auction
    #[payable]
    #[allow(unused_variables)] // TODO: remove when impl done
    pub fn create(
        &mut self,
        asset: AccountId,
        owner_beneficiary: AccountId,
        auction_close_block: Option<BlockHeight>,
        auction_start_bid_amount: Balance,
    ) -> String {
        assert!(
            env::is_valid_account_id(&asset.as_bytes()),
            "Must be a valid root name"
        );
        assert_ne!(
            &asset,
            &env::signer_account_id(),
            "Auction cannot be signer name"
        );

        let close_block = match auction_close_block {
            Some(close_block) => close_block,
            None => env::block_index() + CLOSE_BLOCK_OFFSET,
        };

        let auction = Auction {
            owner_id: env::signer_account_id(),
            asset,
            winner_account_id: None,
            close_block: Some(close_block),
            bids: UnorderedMap::new(env::keccak256(env::block_index().to_string().as_bytes())),
        };
        logger!("auction string: {}", &auction.to_string());
        // Convert our auction to a string & compute the keccak256 hash
        let hash = env::keccak256(&auction.to_string().as_bytes());

        let key: Vec<String> = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Check if there is already an auction with this same matching hash
        // AND if that auction is ongoing (ongoing = current block < closing block)
        let previous_auction = self.auctions.get(&key.join(""));
        match previous_auction {
            Some(previous_auction) => {
                assert!(
                    env::block_index() > previous_auction.close_block.unwrap(),
                    "Auction is already happening"
                );
            }
            None => (),
        }

        self.auctions.insert(&key.join(""), &auction);

        // Use our fancy Macro, because KA CHING!
        logger!("New Auction:{}", &key.join(""));

        // Transfer ownership from ALL previous keys, to the escrow account
        transfer_ownership(
            env::signer_account_id(),
            Base58PublicKey {
                0: env::signer_account_pk(),
            },
            self.escrow_public_key.as_ref().unwrap().clone(),
            self.escrow_account_id.as_ref().unwrap().clone(),
        );

        // Allow original owner to call the cancel auction for their previously owned auction item
        // TODO: Do i need to do this? Or is it just super duper nice because im a nice person?
        Promise::new(env::signer_account_id()).add_access_key(
            env::signer_account_pk(),
            ACCESS_KEY_ALLOWANCE, // TODO: Check this value is right for this FN!
            env::signer_account_id(),
            b"cancel_auction".to_vec(),
        );

        key.join("")
    }

    // return single auction item
    pub fn get_auction_by_id(&self, id: String) -> String {
        // match id {
        //     Some(result) => self.auctions.get(
        //         &result.try_to_vec().unwrap()
        //     ).unwrap(),
        //     None => panic!("Auction ID: {:?} not found", id.as_ref()),
        // }
        self.auctions.get(&id).unwrap().to_string()
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
    pub fn place_bid(&mut self, auction_id: String) -> Promise {
        match self.auctions.get(&auction_id) {
            Some(auction) => {
                assert_ne!(
                    auction.owner_id,
                    env::signer_account_id(),
                    "Must not be owner of auction"
                );
                assert!(
                    env::attached_deposit() > 0,
                    "Must submit bid amount of greater than zero"
                );
                assert!(
                    env::block_index() > auction.close_block.unwrap(),
                    "Must be an active auction"
                );
            }
            None => {
                panic!("Shit got real");
            }
        }

        // TODO: Finish
        // Transfer amount from transaction into the escrow account
        // Annotate how much balance user spent
        Promise::new(self.escrow_account_id.as_ref().unwrap().clone())
            .transfer(env::attached_deposit())
    }

    // removes an auction if owner called it
    // sends back all auction bidders their funds
    pub fn cancel_auction(&mut self, auction_id: String) {
        if let Some(auction) = self.auctions.get(&auction_id) {
            assert_eq!(
                auction.owner_id,
                env::signer_account_id(),
                "Must be owner to cancel auction"
            );

            // TODO: Send bidders their funds

            // remove auction data
            self.auctions.remove(&auction_id);
        } else {
            panic!("Failed to cancel auction")
        }

        // TODO:
        // // Transfer ownership from escrow account, back to the original owner account
        // transfer_ownership(
        //     env::signer_account_id(),
        //     env::signer_account_pk() as Base58PublicKey,
        //     self.escrow_public_key.into(),
        //     self.escrow_account_id
        // );
    }

    // finalize auction:
    // - award winner the asset, if they were highest bidder
    // - all bidders get their bid amounts back, minus fees
    //
    // NOTE: anyone can call this method, as it is paid by the person wanting the final outcome
    pub fn finalize_auction(&mut self, auction_id: String) {
        // TBD!!!!!
        logger!("{}", auction_id);

        // TODO:
        // // Transfer ownership from escrow account, to the new owner account
        // transfer_ownership(
        //     env::signer_account_id(),
        //     env::signer_account_pk() as Base58PublicKey,
        //     self.escrow_public_key.into(),
        //     self.escrow_account_id
        // );
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
            contract.escrow_public_key.unwrap(),
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

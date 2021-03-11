use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::Base58PublicKey;
use near_sdk::{env, near_bindgen, AccountId, Balance, BlockHeight, Promise};
mod util;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000;

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

}

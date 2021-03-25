use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
// use near_sdk::json_types::{ValidAccountId};
use near_sdk::{
    log, near_bindgen, setup_alloc, PanicOnDefault,
};

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Escrow {
}

// TODO: Escrow
// Contract keeps track of accounts in escrow
// is the only account that can execute functions on escrowed account
// should not be owned by anyone
#[near_bindgen]
impl Escrow {
    #[init]
    pub fn new() -> Self {
        Escrow {}
    }

    pub fn thang(&self) -> String {
        let msg = "hiii";
        log!("{}", &msg);
        (&msg).to_string()
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
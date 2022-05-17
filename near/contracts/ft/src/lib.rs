//#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};

use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{Base64VecU8, U128};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise,
    PromiseOrValue, PromiseResult, StorageUsage,
};

use near_sdk::serde_json::Value;

use hex;

// near_sdk::setup_alloc!();

const CHAIN_ID_NEAR: u16 = 15;
const CHAIN_ID_SOL: u16 = 1;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct FTContractMeta {
    metadata: FungibleTokenMetadata,
    vaa: Vec<u8>,
    sequence: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct FTContract {
    token: FungibleToken,
    meta: LazyOption<FTContractMeta>,
    controller: AccountId,
}

#[near_bindgen]
impl FTContract {
//    #[init]
//    pub fn new() -> Self {
//        assert!(!env::state_exists(), "Already initialized");
//        Self {
//            controller: env::predecessor_account_id(),
//            token: FungibleToken::new(b"t".to_vec()),
//            name: String::default(),
//            symbol: String::default(),
//            reference: String::default(),
//            reference_hash: Base64VecU8(vec![]),
//            decimals: 0,
//            paused: Mask::default(),
//            #[cfg(feature = "migrate_icon")]
//            icon: None,
//        }
//    }
//
//    pub fn set_metadata(
//        &mut self,
//        name: Option<String>,
//        symbol: Option<String>,
//        reference: Option<String>,
//        reference_hash: Option<Base64VecU8>,
//        decimals: Option<u8>,
//        icon: Option<String>,
//    ) {
//        // Only owner can change the metadata
//        assert!(self.controller_or_self());
//
//        name.map(|name| self.name = name);
//        symbol.map(|symbol| self.symbol = symbol);
//        reference.map(|reference| self.reference = reference);
//        reference_hash.map(|reference_hash| self.reference_hash = reference_hash);
//        decimals.map(|decimals| self.decimals = decimals);
//        #[cfg(feature = "migrate_icon")]
//        icon.map(|icon| self.icon = Some(icon));
//        #[cfg(not(feature = "migrate_icon"))]
//        icon.map(|_| {
//            env::log("Icon was provided, but it's not supported for the token".as_bytes())
//        });
//    }


    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        env::panic_str("an_account_closed");
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        env::panic_str("on_tokens_burned");
    }

    fn new_ftcontract(
        owner_id: AccountId,
        metadata: FungibleTokenMetadata,
        asset_meta: Vec<u8>,
        seq_number: u64,
    ) -> FTContract {
        metadata.assert_valid();

        let mut ft_vec = Vec::with_capacity(64);
        ft_vec.extend(b"ft".to_vec());
        ft_vec.extend(&*asset_meta);
        ft_vec.extend(owner_id.as_bytes());

        let mut md_vec = Vec::with_capacity(64);
        md_vec.extend(b"md".to_vec());
        md_vec.extend(&*asset_meta);
        md_vec.extend(owner_id.as_bytes());

        let meta = FTContractMeta {
            metadata: metadata,
            vaa: asset_meta,
            sequence: seq_number,
        };

        let this = FTContract {
            token: FungibleToken::new(env::sha256(&ft_vec)),
            meta: LazyOption::new(env::sha256(&md_vec), Some(&meta)),
            controller: env::predecessor_account_id(),
        };
        this
    }

//    #[payable]
//    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
//        assert_eq!(
//            env::predecessor_account_id(),
//            self.controller,
//            "Only controller can call mint"
//        );
//
//        self.storage_deposit(Some(account_id.as_str().try_into().unwrap()), None);
//        self.token.internal_deposit(&account_id, amount.into());
//    }
//
//    #[payable]
//    pub fn withdraw(&mut self, amount: U128, recipient: String) -> Promise {
//        self.check_not_paused(PAUSE_WITHDRAW);
//
//        assert_one_yocto();
//        Promise::new(env::predecessor_account_id()).transfer(1);
//
//        self.token
//            .internal_withdraw(&env::predecessor_account_id(), amount.into());
//
//        ext_bridge_token_factory::finish_withdraw(
//            amount.into(),
//            recipient,
//            &self.controller,
//            NO_DEPOSIT,
//            FINISH_WITHDRAW_GAS,
//        )
//    }

    pub fn account_storage_usage(&self) -> StorageUsage {
        self.token.account_storage_usage
    }

    /// Return true if the caller is either controller or self
    pub fn controller_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        caller == self.controller || caller == env::current_account_id()
    }

}

near_contract_standards::impl_fungible_token_core!(FTContract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(FTContract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for FTContract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.meta.get().unwrap().metadata
    }
}

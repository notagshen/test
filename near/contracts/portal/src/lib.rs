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
    PromiseOrValue, PromiseResult,
};

use near_sdk::serde_json::Value;

use hex;

pub mod byte_utils;
pub mod state;

use crate::byte_utils::{get_string_from_32, ByteUtils};

// near_sdk::setup_alloc!();

const CHAIN_ID_NEAR: u16 = 15;
const CHAIN_ID_SOL: u16 = 1;

#[ext_contract(ext_core_bridge)]
pub trait CoreBridge {
    fn verify_vaa(&self, vaa: String) -> (String, i32);
    fn publish_message(&self, data: Vec<u8>) -> u64;
}

#[ext_contract(ext_self)]
pub trait TokenBridgeCallback {
    fn submit_vaa_callback(&mut self);
}

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
}

#[near_bindgen]
impl FTContract {
    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        env::panic_str("an_account_closed");
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        env::panic_str("on_tokens_burned");
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
    };
    this
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenBridge {
    dups: UnorderedSet<Vec<u8>>,
    contracts: LookupMap<u16, Vec<u8>>,
    tokens: LookupMap<Vec<u8>, FTContract>,
    booted: bool,
    core: AccountId,
}

impl Default for TokenBridge {
    fn default() -> Self {
        Self {
            dups: UnorderedSet::new(b"d".to_vec()),
            contracts: LookupMap::new(b"c".to_vec()),
            tokens: LookupMap::new(b"t".to_vec()),
            booted: false,
            core: AccountId::new_unchecked("".to_string()),
        }
    }
}

fn vaa_register_chain(storage: &mut TokenBridge, vaa: state::ParsedVAA) {
    let data: &[u8] = &vaa.payload;
    let chain = data.get_u16(33);

    if (chain != CHAIN_ID_NEAR) && (chain != 0) {
        env::panic_str("InvalidRegisterChainChain");
    }

    if storage.contracts.contains_key(&chain) {
        env::panic_str("DuplicateChainRegistration");
    }

    storage.contracts.insert(&chain, &data[34..66].to_vec());
}

fn vaa_upgrade_contract(storage: &mut TokenBridge, vaa: state::ParsedVAA) {
    let data: &[u8] = &vaa.payload;
    let chain = data.get_u16(33);
    if chain != CHAIN_ID_NEAR {
        env::panic_str("InvalidContractUpgradeChain");
    }

    env::panic_str("ContractUpgradesNotImplemented");
}

fn vaa_governance(storage: &mut TokenBridge, vaa: state::ParsedVAA, gov_idx: u32) {
    if gov_idx != vaa.guardian_set_index {
        env::panic_str("InvalidGovernanceSet");
    }

    if (CHAIN_ID_SOL != vaa.emitter_chain)
        || (hex::decode("0000000000000000000000000000000000000000000000000000000000000004")
            .unwrap()
            != vaa.emitter_address)
    {
        env::panic_str("InvalidGovernanceEmitter");
    }

    let data: &[u8] = &vaa.payload;
    let action = data.get_u8(32);

    match action {
        1u8 => vaa_register_chain(storage, vaa),
        2u8 => vaa_upgrade_contract(storage, vaa),
        _ => env::panic_str("InvalidGovernanceAction"),
    }
}

fn vaa_transfer(storage: &mut TokenBridge, vaa: state::ParsedVAA) {
    let data: &[u8] = &vaa.payload[1..];
    let amount = data.get_u256(0);
    let token_address = data.get_bytes32(32).to_vec();
    let token_chain = data.get_u16(64);
    let recipient = data.get_bytes32(66).to_vec();
    let recipient_chain = data.get_u16(98);
    let fee = data.get_u256(100);

    env::panic_str("vaa_transfer");
}

fn vaa_asset_meta(storage: &mut TokenBridge, vaa: state::ParsedVAA) {
    let data: &[u8] = &vaa.payload[1..];

    let tkey = data[0..34].to_vec();
    //let token_address = data.get_bytes32(0).to_vec();
    let token_chain = data.get_u16(32);
    let mut decimals = data.get_u8(34);
    let symbol = data.get_bytes32(35).to_vec();
    let name = data.get_bytes32(67).to_vec();

    if token_chain == CHAIN_ID_NEAR {
        env::panic_str("CannotAttestNearAssets");
    }

    let wname = get_string_from_32(&name) + " (Wormhole)";

    if storage.tokens.contains_key(&tkey) {
        let mut ft = storage.tokens.get(&tkey).unwrap();
        let mut md = ft.meta.get().unwrap();

        if md.sequence > vaa.sequence {
            env::panic_str("ExpiredAssetMetaVaa");
        }
        md.sequence = vaa.sequence;

        md.metadata.name = wname;
        md.metadata.symbol = get_string_from_32(&symbol);
        ft.meta.replace(&md);

        return;
    }

    // Decimals are capped at 8 in wormhole
    if decimals > 8 {
        decimals = 8;
    }

    // Stick some useful meta-data into the asset to allow us to map backwards from a on-chain asset to the wormhole meta data
    let reference = near_sdk::base64::encode(&tkey);
    let ref_hash = env::sha256(&reference.as_bytes().to_vec());

    let ft = FungibleTokenMetadata {
        spec: FT_METADATA_SPEC.to_string(),
        name: wname,
        symbol: get_string_from_32(&symbol),
        icon: Some("".to_string()), // Is there ANY way to supply this?
        reference: Some(reference.clone()),
        reference_hash: Some(Base64VecU8::from(ref_hash)),
        decimals: decimals,
    };

    let mut token = new_ftcontract(env::current_account_id(), ft, data.to_vec(), vaa.sequence);

    storage.tokens.insert(&tkey, &token);

    token.storage_deposit(None, None);
}

fn vaa_transfer_with_payload(storage: &mut TokenBridge, vaa: state::ParsedVAA) {
    let data: &[u8] = &vaa.payload[1..];
    let amount = data.get_u256(0);
    let token_address = data.get_bytes32(32).to_vec();
    let token_chain = data.get_u16(64);
    let recipient = data.get_bytes32(66).to_vec();
    let recipient_chain = data.get_u16(98);
    let fee = data.get_u256(100);
    let payload = &data[132..];

    env::panic_str("vaa_transfer_with_payload");
}

#[near_bindgen]
impl TokenBridge {
    pub fn submit_vaa(&mut self, vaa: String) -> Promise {
        ext_core_bridge::verify_vaa(
            vaa,
            self.core.clone(),        // contract account id
            0,                        // yocto NEAR to attach
            Gas(100_000_000_000_000), // gas to attach
        )
        .then(ext_self::submit_vaa_callback(
            env::current_account_id(), // me
            0,                         // yocto NEAR to attach to the callback
            Gas(100_000_000_000_000),  // gas to attach
        ))
    }

    #[private] // So, all of wormhole security rests in this one statement?
    pub fn submit_vaa_callback(&mut self) {
        // well, and this one...
        if (env::promise_results_count() != 1)
            || (env::predecessor_account_id() != env::current_account_id())
        {
            env::panic_str("BadPredecessorAccount");
        }

        let data: String;
        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                data = String::from_utf8(result).unwrap();
            }
            _ => env::panic_str("vaaVerifyFail"),
        }

        let v: Value = near_sdk::serde_json::from_str(&data).unwrap();

        // Please, what is the correct way of just getting a fricken string?!
        let _vaa = v[0].to_string();
        let vaa = &_vaa[1.._vaa.len() - 1];

        let gov_idx = v[1].as_i64().unwrap() as u32;

        let h = hex::decode(vaa).expect("invalidVaa");

        let vaa = state::ParsedVAA::parse(&h);

        if vaa.version != 1 {
            env::panic_str("InvalidVersion");
        }

        // Check if VAA with this hash was already accepted
        if self.dups.contains(&vaa.hash) {
            env::panic_str("alreadyExecuted");
        }
        self.dups.insert(&vaa.hash);

        let data: &[u8] = &vaa.payload;

        if data[0..32]
            == hex::decode("000000000000000000000000000000000000000000546f6b656e427269646765")
                .unwrap()
        {
            vaa_governance(self, vaa, gov_idx);
            return;
        }

        let action = data.get_u8(0);

        match action {
            1u8 => vaa_transfer(self, vaa),
            2u8 => vaa_asset_meta(self, vaa),
            3u8 => vaa_transfer_with_payload(self, vaa),
            _ => env::panic_str("InvalidPortalAction"),
        }
    }

    pub fn boot_portal(&mut self, core: String) {
        if self.booted {
            env::panic_str("no donut");
        }
        self.booted = true;
        self.core = AccountId::try_from(core.clone()).unwrap();
    }
}

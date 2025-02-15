use cosmwasm_std::{coins, to_json_binary};
use cw20::msg::Cw20ExecuteMsgFns;
use cw20::MinterResponse;
use cw721::TokensResponse;
use cw721_base::interface::Cw721;
use cw_orch::environment::ChainInfoOwned;
use cw_orch::{interface, prelude::*};
use cw_plus_interface::cw20_base::{self, Cw20Base};

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::{MinterExecuteMsgFns, MinterQueryMsgFns};

pub const CONTRACT_ID: &str = "counter_contract";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct MinterContract;

impl<Chain> Uploadable for MinterContract<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw721_minter")
            .unwrap()
    }

    // QUEST #2.1
    // Registering all the endpoints on the smart-contract
    // the cw721_minter smart-contract has a reply endpoint registered
    // It will be included in the wasm by default
    // However, in order to be able to test this capability, the reply endpoint should also be registered on the contract wrapper
    // In this quest, you need to make sure the wrapper has the `reply` endpoint registered
    // To make sure this works, run `cargo test --test 2-1-reply-endpoint` and make sure the test succeeds
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            )
            .with_migrate(crate::contract::migrate),
        )
    }
}

// Quest #3.1
// In this quest, i have copy-pasted the test that was created in Quest 2.
// You will now make it more generic to be able to execute an any environment
// The goal of this quest is to provide the generic_test function to our testing environment.
// In its current state, the function creates an `MockBech32` execution environment and does all the action on this environment.
// We want all those actions to be executed on the `chain` environment instead.
// This `chain` environment is defined as any environment that implemented `CwEnv`.
// `CwEnv` is a trait that signals that everything a `cw-orch` environment needs is implemented.
// For instance, `MockBech32` implements the `CwEnv` trait
// Your first step is to delete the line under the TODO comment
pub fn generic_test<Chain: CwEnv>(
    chain: Chain,
    native_denom: String,
) -> cw_orch::anyhow::Result<()> {
    // TODO : This line should be deleted at the beginning of the quest !
    let mock = MockBech32::new("mock");

    let cw721 = Cw721::new("nft", mock.clone());
    cw721.upload()?;

    let cw20 = Cw20Base::new("cw20", mock.clone());
    cw20.instantiate(
        &cw20_base::InstantiateMsg {
            name: "cw20-test".to_string(),
            symbol: "CWORCH".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: mock.sender().to_string(),
                cap: None,
            }),
            marketing: None,
        },
        None,
        None,
    )?;
    cw20.mint(150_000u128.into(), mock.sender().to_string())?;

    let minter = MinterContract::new(mock.clone());
    minter.upload()?;
    minter.instantiate(
        &InstantiateMsg {
            native_denom: native_denom.to_string(),
            native_price: 200u128.into(),
            cw20_address: cw20.address()?.to_string(),
            cw20_price: 1500u128.into(),
            nft_code_id: cw721.code_id()?,
        },
        None,
        None,
    )?;
    let state = minter.state()?;
    cw721.set_address(&Addr::unchecked(state.nft_address));

    minter.mint(&coins(200, native_denom))?;

    let minted: TokensResponse = cw721.query(&cw721_base::QueryMsg::AllTokens {
        start_after: None,
        limit: None,
    })?;
    assert_eq!(minted.tokens.len(), 1);

    // We mint another NFT but it needs to advance blocks to be able to mint
    mock.wait_blocks(1)?;
    cw20.send(
        1500u128.into(),
        minter.address()?.to_string(),
        to_json_binary(&ExecuteMsg::Mint {})?,
    )?;

    let minted: TokensResponse = cw721.query(&cw721_base::QueryMsg::AllTokens {
        start_after: None,
        limit: None,
    })?;
    assert_eq!(minted.tokens.len(), 2);

    Ok(())
}

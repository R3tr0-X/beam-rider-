use beamrider_backend::{
    chains::cctp::{
        HookAction, HookData, address_to_bytes32, bytes32_to_address, decode_hook_data,
        encode_hook_data,
    },
    domain::{ApyQuote, Venue},
    services::{RebalanceService, StrategyService},
    state::AppState,
};

#[test]
fn cctp_address_and_hook_data_encoding_round_trip() {
    let address = "0x1111111111111111111111111111111111111111";
    let encoded = address_to_bytes32(address).unwrap();
    assert_eq!(&encoded[..12], &[0_u8; 12]);
    assert_eq!(bytes32_to_address(&encoded), address);

    let hook = HookData {
        action: HookAction::DepositAave,
        destination_vault: address.to_string(),
        metadata: b"aave-arbitrum".to_vec(),
    };
    let bytes = encode_hook_data(&hook).unwrap();
    let decoded = decode_hook_data(&bytes).unwrap();
    assert_eq!(decoded, hook);
}

#[test]
fn strategy_selection_uses_net_apy_and_deterministic_tie_breaking() {
    let quotes = vec![
        ApyQuote {
            venue: Venue::AaveArbitrum,
            chain_id: 42161,
            gross_apy_bps: 500,
            estimated_gas_bps: 20,
        },
        ApyQuote {
            venue: Venue::AaveBase,
            chain_id: 8453,
            gross_apy_bps: 490,
            estimated_gas_bps: 5,
        },
        ApyQuote {
            venue: Venue::AaveCelo,
            chain_id: 42220,
            gross_apy_bps: 510,
            estimated_gas_bps: 25,
        },
    ];

    let best = StrategyService::choose_best(&quotes).unwrap();
    assert_eq!(best.venue, Venue::AaveBase);

    let tie = vec![
        ApyQuote {
            venue: Venue::MoolaCelo,
            chain_id: 42220,
            gross_apy_bps: 500,
            estimated_gas_bps: 0,
        },
        ApyQuote {
            venue: Venue::AaveBase,
            chain_id: 8453,
            gross_apy_bps: 500,
            estimated_gas_bps: 0,
        },
    ];
    let best_tie = StrategyService::choose_best(&tie).unwrap();
    assert_eq!(best_tie.venue, Venue::AaveBase);
}

#[tokio::test]
async fn rebalance_planning_persists_plan_and_hook_data() {
    let state = AppState::for_test().await.unwrap();
    let service = RebalanceService::new(state.event_repo.clone());
    let quotes = vec![ApyQuote {
        venue: Venue::AaveArbitrum,
        chain_id: 42161,
        gross_apy_bps: 520,
        estimated_gas_bps: 20,
    }];

    let planned = service
        .plan_best(
            42220,
            "1000000",
            &quotes,
            "0x1111111111111111111111111111111111111111",
        )
        .await
        .unwrap();

    assert_eq!(planned.id, 1);
    assert_eq!(planned.plan.expected_apy_bps, 500);
    assert!(!planned.hook_data.is_empty());
    assert_eq!(state.event_repo.count_rebalances().await.unwrap(), 1);
}

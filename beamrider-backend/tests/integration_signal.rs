use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::Engine;
use beamrider_backend::{
    crypto::ResponseSigner,
    domain::{MarketSignal, SignalKind, VerifiedPayment, normalize_pair},
    dto::{AgentStatusResponse, FixturePaymentHeader, SignalResponse},
    error::AppError,
    routes,
    services::SignalPayment,
    state::AppState,
};
use chrono::{DateTime, Utc};
use tower::ServiceExt;

#[test]
fn canonical_signal_bytes_are_stable_and_pairs_are_validated() {
    let created_at = DateTime::parse_from_rfc3339("2026-05-09T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let signal = MarketSignal::new(" eth-usd ", SignalKind::Buy, 125, 0.75, created_at).unwrap();

    assert_eq!(signal.pair, "ETH-USD");
    assert_eq!(
        signal.canonical_bytes(),
        b"beamrider.signal.v1\npair=ETH-USD\nkind=BUY\nvalue_bps=125\nconfidence_ppm=750000\ncreated_at=2026-05-09T00:00:00.000Z"
    );
    assert_eq!(normalize_pair("btc-usdc").unwrap(), "BTC-USDC");
    assert!(normalize_pair("").is_err());
    assert!(normalize_pair("ETH").is_err());
    assert!(normalize_pair("ETH-USD-EXTRA").is_err());
    assert!(normalize_pair("ETH/USD").is_err());
}

#[test]
fn ed25519_signatures_verify_and_reject_modified_payloads() {
    let signer = ResponseSigner::from_optional_secret(None).unwrap();
    let message = b"beamrider-test-message";
    let signature = signer.sign(message);
    let public_key = signer.public_key_bytes();

    ResponseSigner::verify(&public_key, message, &signature).unwrap();
    assert!(ResponseSigner::verify(&public_key, b"tampered", &signature).is_err());
}

#[tokio::test]
async fn sqlite_repositories_round_trip_signals_and_reject_duplicate_sales() {
    let state = AppState::for_test().await.unwrap();
    let signer = ResponseSigner::from_optional_secret(None).unwrap();
    let signal = MarketSignal::new("ETH-USD", SignalKind::Hold, 0, 0.6, Utc::now()).unwrap();
    let signature = signer.sign(&signal.canonical_bytes());

    let id = state
        .signal_repo
        .insert_signed(&signal, &signature)
        .await
        .unwrap();
    assert_eq!(id, 1);
    let history = state
        .signal_repo
        .last_n_for_pair("eth-usd", 5)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].signal.pair, "ETH-USD");

    let payment = verified_payment("0xdup");
    state
        .signal_service
        .produce("ETH-USD", SignalPayment::X402(payment.clone()))
        .await
        .unwrap();
    let err = state
        .signal_service
        .produce("ETH-USD", SignalPayment::X402(payment))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));
}

#[tokio::test]
async fn paid_signal_route_produces_signed_persisted_response_and_status() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state.clone());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header("x-payment", fixture_header("0xroute1", "100000"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: SignalResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.pair, "ETH-USD");

    let signature = hex::decode(&body.signature).unwrap();
    let public_key = hex::decode(&body.public_key).unwrap();
    let signal = MarketSignal::new(
        &body.pair,
        body.kind,
        body.value_bps,
        body.confidence,
        body.created_at,
    )
    .unwrap();
    ResponseSigner::verify(&public_key, &signal.canonical_bytes(), &signature).unwrap();

    let status_response = app
        .oneshot(
            Request::builder()
                .uri("/v1/agent/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_bytes = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: AgentStatusResponse = serde_json::from_slice(&status_bytes).unwrap();
    assert_eq!(status.signal_count, 1);
    assert!(status.latest_signal.is_some());
}

fn verified_payment(tx_hash: &str) -> VerifiedPayment {
    VerifiedPayment {
        buyer: "0x1111111111111111111111111111111111111111".to_string(),
        chain_id: 8453,
        network: "eip155:8453".to_string(),
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
        amount_atoms: "100000".to_string(),
        tx_hash: tx_hash.to_string(),
        settled_at: Utc::now(),
    }
}

fn fixture_header(tx_hash: &str, amount_atoms: &str) -> String {
    let header = FixturePaymentHeader {
        buyer: "0x1111111111111111111111111111111111111111".to_string(),
        chain_id: 8453,
        network: "eip155:8453".to_string(),
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
        amount_atoms: amount_atoms.to_string(),
        tx_hash: tx_hash.to_string(),
    };
    let json = serde_json::to_vec(&header).unwrap();
    format!(
        "fixture:{}",
        base64::engine::general_purpose::STANDARD.encode(json)
    )
}

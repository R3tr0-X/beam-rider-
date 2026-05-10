use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::Engine;
use beamrider_backend::{
    dto::{FixturePaymentHeader, IssueSessionResponse, SignalResponse},
    routes,
    state::AppState,
};
use tower::ServiceExt;

#[tokio::test]
async fn session_voucher_round_trip_decrements_balance_and_rejects_exhausted() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state);

    // Buy a session voucher with a one-shot x402 payment.
    let issue = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/sessions")
                .header("x-payment", fixture_header("0xsession-mint", "100000"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"requests":2}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(issue.status(), StatusCode::OK);
    let issued: IssueSessionResponse =
        serde_json::from_slice(&to_bytes(issue.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(issued.balance, 2);
    assert!(!issued.token.is_empty());

    // First redemption succeeds.
    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header("x402-session", &issued.token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let body: SignalResponse =
        serde_json::from_slice(&to_bytes(first.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body.pair, "ETH-USD");

    // Second redemption succeeds (balance was 2).
    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header("x402-session", &issued.token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    // Third redemption is rejected — voucher is exhausted.
    let third = app
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header("x402-session", &issued.token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unknown_session_token_is_rejected() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header("x402-session", "this-token-does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn minipay_disabled_in_test_config_rejects_with_402_path() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header(
                    "x-minipay-tx-hash",
                    "0x1111111111111111111111111111111111111111111111111111111111111111",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // CeloPaymentVerifier reports not-enabled; mapped to 403 PaymentVerification.
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stacks_header_without_verifier_is_rejected() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .header(
                    "x-stacks-tx-id",
                    "0x2222222222222222222222222222222222222222222222222222222222222222",
                )
                .header(
                    "x-stacks-buyer",
                    "SP1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                )
                .header("x-stacks-token", "stx")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
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

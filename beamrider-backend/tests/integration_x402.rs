use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Request, StatusCode},
};
use base64::Engine;
use beamrider_backend::{
    dto::{FixturePaymentHeader, PaymentRequiredResponse},
    middleware::x402::decode_header_payload,
    routes,
    state::AppState,
};
use tower::ServiceExt;

#[tokio::test]
async fn unpaid_signal_request_returns_402_with_payment_requirements() {
    let state = AppState::for_test().await.unwrap();
    let app = routes::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/signals/ETH-USD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: PaymentRequiredResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.x402_version, 2);
    assert_eq!(body.accepts.len(), 2);
    assert!(
        body.accepts
            .iter()
            .all(|requirement| requirement.scheme == "exact")
    );
}

#[tokio::test]
async fn fixture_verifier_accepts_valid_payment_and_rejects_malformed_payment() {
    let state = AppState::for_test().await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-payment",
        HeaderValue::from_str(&fixture_header("0xabc", "100000")).unwrap(),
    );
    let payment = state
        .x402
        .verify_headers(&headers, "ETH-USD")
        .await
        .unwrap();
    assert_eq!(payment.chain_id, 8453);
    assert_eq!(payment.tx_hash, "0xabc");

    let mut malformed = HeaderMap::new();
    malformed.insert("x-payment", HeaderValue::from_static("not-base64-json"));
    assert!(
        state
            .x402
            .verify_headers(&malformed, "ETH-USD")
            .await
            .is_err()
    );

    let value = decode_header_payload(&fixture_header("0xabc", "100000")).unwrap();
    assert_eq!(value["tx_hash"], "0xabc");
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
    format!(
        "fixture:{}",
        base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&header).unwrap())
    )
}

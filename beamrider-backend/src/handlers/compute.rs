use axum::{Json, extract::State};
use chrono::{SecondsFormat, Utc};

use crate::domain::SignatureEnvelope;
use crate::dto::{ComputeRequest, ComputeResponse};
use crate::error::AppError;
use crate::state::AppState;

pub async fn compute(
    State(state): State<AppState>,
    Json(request): Json<ComputeRequest>,
) -> Result<Json<ComputeResponse>, AppError> {
    if request.input.trim().is_empty() {
        return Err(AppError::BadRequest("input is required".to_string()));
    }
    let created_at = Utc::now();
    let context = request.context.clone();
    let canonical = format!(
        "beamrider.compute.v1\ninput={}\ncontext={}\ncreated_at={}",
        request.input,
        context
            .as_ref()
            .map_or_else(|| "null".to_string(), serde_json::Value::to_string),
        created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
    );
    let signature = state.signer.sign(canonical.as_bytes());
    let public_key = state.signer.public_key_bytes();

    Ok(Json(ComputeResponse {
        input: request.input.clone(),
        digest: stable_digest(canonical.as_bytes()),
        created_at,
        attestation: SignatureEnvelope::new(&signature, &public_key),
        context,
    }))
}

fn stable_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

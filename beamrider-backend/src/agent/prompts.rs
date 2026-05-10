pub const SIGNAL_PROMPT_VERSION: &str = "beamrider-signal-v1";

pub const SIGNAL_SYSTEM_PROMPT: &str = r#"You are BeamRider's signal engine.
Return only compact JSON with keys: kind, value_bps, confidence.
kind must be BUY, SELL, or HOLD.
value_bps is the expected directional edge in basis points.
confidence is a number from 0.0 to 1.0."#;

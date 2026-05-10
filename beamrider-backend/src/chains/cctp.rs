use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookAction {
    DepositAave = 1,
    ReturnHome = 2,
}

impl HookAction {
    pub fn from_u8(value: u8) -> Result<Self, AppError> {
        match value {
            1 => Ok(Self::DepositAave),
            2 => Ok(Self::ReturnHome),
            _ => Err(AppError::BadRequest(format!(
                "unknown hook action: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookData {
    pub action: HookAction,
    pub destination_vault: String,
    pub metadata: Vec<u8>,
}

pub fn address_to_bytes32(address: &str) -> Result<[u8; 32], AppError> {
    let bytes = parse_address(address)?;
    let mut out = [0_u8; 32];
    out[12..].copy_from_slice(&bytes);
    Ok(out)
}

pub fn bytes32_to_address(bytes: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(&bytes[12..]))
}

pub fn encode_hook_data(data: &HookData) -> Result<Vec<u8>, AppError> {
    let mut out = Vec::with_capacity(1 + 32 + 4 + data.metadata.len());
    out.push(data.action as u8);
    out.extend_from_slice(&address_to_bytes32(&data.destination_vault)?);
    let metadata_len: u32 = data
        .metadata
        .len()
        .try_into()
        .map_err(|_| AppError::BadRequest("metadata too large".to_string()))?;
    out.extend_from_slice(&metadata_len.to_be_bytes());
    out.extend_from_slice(&data.metadata);
    Ok(out)
}

pub fn decode_hook_data(bytes: &[u8]) -> Result<HookData, AppError> {
    if bytes.len() < 37 {
        return Err(AppError::BadRequest("hook data too short".to_string()));
    }
    let action = HookAction::from_u8(bytes[0])?;
    let mut address = [0_u8; 32];
    address.copy_from_slice(&bytes[1..33]);
    let mut len_bytes = [0_u8; 4];
    len_bytes.copy_from_slice(&bytes[33..37]);
    let metadata_len = u32::from_be_bytes(len_bytes) as usize;
    if bytes.len() != 37 + metadata_len {
        return Err(AppError::BadRequest(
            "hook metadata length mismatch".to_string(),
        ));
    }
    Ok(HookData {
        action,
        destination_vault: bytes32_to_address(&address),
        metadata: bytes[37..].to_vec(),
    })
}

fn parse_address(address: &str) -> Result<[u8; 20], AppError> {
    let trimmed = address.trim();
    let hex_value = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex_value.len() != 40 || !hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::BadRequest(
            "address must be 20 hex bytes".to_string(),
        ));
    }
    let decoded = hex::decode(hex_value).map_err(|err| AppError::BadRequest(err.to_string()))?;
    decoded
        .try_into()
        .map_err(|_| AppError::BadRequest("address must be 20 bytes".to_string()))
}

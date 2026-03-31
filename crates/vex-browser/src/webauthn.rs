// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! WebAuthn core types (browser-side scaffolding).
//!
//! This module defines strongly-typed request/response models used by
//! the browser shell and platform authenticator providers.
//! Platform-specific calls (Windows Hello, Touch ID) are implemented in
//! separate layers.

/// Error type for WebAuthn request validation / execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebAuthnError {
    /// Request content is malformed.
    InvalidRequest(String),
    /// Current platform authenticator/provider cannot satisfy request.
    NotSupported(String),
    /// User cancelled authentication flow.
    Cancelled,
    /// Provider/internal failure.
    Internal(String),
}

impl std::fmt::Display for WebAuthnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "InvalidRequest: {msg}"),
            Self::NotSupported(msg) => write!(f, "NotSupportedError: {msg}"),
            Self::Cancelled => write!(f, "AbortError: operation cancelled"),
            Self::Internal(msg) => write!(f, "OperationError: {msg}"),
        }
    }
}

/// How authenticator attachment should be constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticatorAttachment {
    Platform,
    CrossPlatform,
}

/// User verification policy for credential operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserVerificationRequirement {
    Required,
    Preferred,
    Discouraged,
}

/// Attestation conveyance preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationConveyancePreference {
    None,
    Indirect,
    Direct,
    Enterprise,
}

/// Relying Party metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelyingParty {
    pub id: String,
    pub name: String,
}

/// User metadata for credential creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyUser {
    pub id: Vec<u8>,
    pub name: String,
    pub display_name: String,
}

/// Supported credential algorithm (COSE `alg` value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKeyCredentialParameter {
    pub alg: i32,
}

/// Credential descriptor for allow/exclude lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialDescriptor {
    pub id: Vec<u8>,
}

/// Public key credential creation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCredentialRequest {
    pub rp: RelyingParty,
    pub user: PublicKeyUser,
    pub challenge: Vec<u8>,
    pub pub_key_cred_params: Vec<PublicKeyCredentialParameter>,
    pub timeout_ms: Option<u32>,
    pub attestation: AttestationConveyancePreference,
    pub authenticator_attachment: Option<AuthenticatorAttachment>,
    pub user_verification: UserVerificationRequirement,
    pub exclude_credentials: Vec<CredentialDescriptor>,
}

/// Public key credential assertion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAssertionRequest {
    pub challenge: Vec<u8>,
    pub rp_id: Option<String>,
    pub timeout_ms: Option<u32>,
    pub user_verification: UserVerificationRequirement,
    pub allow_credentials: Vec<CredentialDescriptor>,
}

/// Simplified credential response produced by platform providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialResponse {
    pub credential_id: Vec<u8>,
    pub client_data_json: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    pub user_handle: Option<Vec<u8>>,
}

/// Validate minimum request properties before provider invocation.
pub fn validate_challenge(challenge: &[u8]) -> Result<(), WebAuthnError> {
    // Conservative range; avoids trivially weak / malformed inputs.
    // (Spec allows variable sizes; browser-side policy can enforce stronger defaults.)
    if challenge.len() < 16 {
        return Err(WebAuthnError::InvalidRequest(
            "challenge must be at least 16 bytes".to_owned(),
        ));
    }
    if challenge.len() > 1024 {
        return Err(WebAuthnError::InvalidRequest(
            "challenge must be <= 1024 bytes".to_owned(),
        ));
    }
    Ok(())
}

/// Validate RP ID shape (basic browser-side gate).
pub fn validate_rp_id(rp_id: &str) -> Result<(), WebAuthnError> {
    let id = rp_id.trim();
    if id.is_empty() {
        return Err(WebAuthnError::InvalidRequest(
            "rp.id cannot be empty".to_owned(),
        ));
    }
    if id.contains(' ') {
        return Err(WebAuthnError::InvalidRequest(
            "rp.id cannot contain spaces".to_owned(),
        ));
    }
    if !id.contains('.') && id != "localhost" {
        return Err(WebAuthnError::InvalidRequest(
            "rp.id must be a registrable domain or localhost".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_validation_accepts_reasonable_length() {
        let challenge = vec![0xAB; 32];
        assert!(validate_challenge(&challenge).is_ok());
    }

    #[test]
    fn challenge_validation_rejects_short() {
        let challenge = vec![0xAA; 8];
        let err = validate_challenge(&challenge).unwrap_err();
        assert!(matches!(err, WebAuthnError::InvalidRequest(_)));
    }

    #[test]
    fn challenge_validation_rejects_oversized() {
        let challenge = vec![0xCC; 2000];
        let err = validate_challenge(&challenge).unwrap_err();
        assert!(matches!(err, WebAuthnError::InvalidRequest(_)));
    }

    #[test]
    fn rp_id_validation() {
        assert!(validate_rp_id("example.com").is_ok());
        assert!(validate_rp_id("login.vigo.dev").is_ok());
        assert!(validate_rp_id("localhost").is_ok());

        assert!(validate_rp_id("").is_err());
        assert!(validate_rp_id("example com").is_err());
        assert!(validate_rp_id("invalid").is_err());
    }
}

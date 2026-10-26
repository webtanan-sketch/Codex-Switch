//! Read-only Codex authentication adapter. Payload bytes are never reserialized.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(
        "Authentication is missing, malformed, or unsupported. Import a ChatGPT login from Codex."
    )]
    Invalid,
    #[error("No stable user identity was found. This Codex auth format is not supported.")]
    Identity,
    #[error("The Codex configuration is invalid; no changes were made.")]
    Config,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub fingerprint: String,
    pub email: Option<String>,
    pub plan: Option<String>,
}
// Deliberately no Debug/Serialize: secrets must never reach commands, logs, or IPC.
pub struct AuthPayload {
    bytes: Zeroizing<Vec<u8>>,
    pub identity: Identity,
}
fn text_at<'a>(v: &'a Value, pointers: &[&str]) -> Option<&'a str> {
    pointers
        .iter()
        .find_map(|p| v.pointer(p)?.as_str().filter(|s| !s.trim().is_empty()))
}
fn safe_label(s: &str) -> Option<String> {
    (s.len() <= 254 && !s.chars().any(char::is_control)).then(|| s.to_owned())
}
impl AuthPayload {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, AuthError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() > 1024 * 1024 {
            return Err(AuthError::Invalid);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| AuthError::Invalid)?;
        let tokens = value
            .get("tokens")
            .filter(|v| v.is_object())
            .ok_or(AuthError::Invalid)?;
        for field in ["access_token", "refresh_token"] {
            if tokens
                .get(field)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .is_none()
            {
                return Err(AuthError::Invalid);
            }
        }
        // Claims are unverified, local identity hints only, never an authorization decision.
        // No signature/token modification, refresh requests, or token manufacturing.
        let claims = tokens
            .get("id_token")
            .and_then(Value::as_str)
            .and_then(|t| t.split('.').nth(1))
            .and_then(|p| URL_SAFE_NO_PAD.decode(p).ok())
            .and_then(|p| serde_json::from_slice::<Value>(&p).ok())
            .unwrap_or(Value::Null);
        let user = text_at(
            &claims,
            &["/https:~1~1api.openai.com~1auth/chatgpt_user_id", "/sub"],
        )
        .or_else(|| text_at(&value, &["/userId", "/chatgptUserId", "/tokens/user_id"]));
        // Workspace IDs alone cannot distinguish two people in the same organization.
        let user = user.ok_or(AuthError::Identity)?;
        let account = text_at(&value, &["/tokens/account_id", "/accountId"])
            .or_else(|| {
                text_at(
                    &claims,
                    &["/https:~1~1api.openai.com~1auth/chatgpt_account_id"],
                )
            })
            .unwrap_or("");
        let mut hash = Sha256::new();
        for part in ["codex-account-manager.identity.v1", user, account] {
            hash.update((part.len() as u64).to_le_bytes());
            hash.update(part.as_bytes());
        }
        let identity = Identity {
            fingerprint: format!("{:x}", hash.finalize()),
            email: text_at(&claims, &["/email"]).and_then(safe_label),
            plan: text_at(
                &claims,
                &["/https:~1~1api.openai.com~1auth/chatgpt_plan_type"],
            )
            .and_then(safe_label),
        };
        Ok(Self { bytes, identity })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> String {
        format!("{:x}", Sha256::digest(&*self.bytes))
    }
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CredentialStore {
    File,
    Keyring,
    Auto,
    Ephemeral,
    Unspecified,
}
pub fn storage_mode(config: &str) -> Result<CredentialStore, AuthError> {
    let doc = config
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| AuthError::Config)?;
    Ok(
        match doc
            .get("cli_auth_credentials_store")
            .and_then(|v| v.as_str())
        {
            Some("file") => CredentialStore::File,
            Some("keyring") => CredentialStore::Keyring,
            Some("auto") => CredentialStore::Auto,
            Some("ephemeral") => CredentialStore::Ephemeral,
            None => CredentialStore::Unspecified,
            _ => return Err(AuthError::Config),
        },
    )
}
pub fn file_storage_config(config: &str) -> Result<String, AuthError> {
    let mut doc = config
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| AuthError::Config)?;
    doc["cli_auth_credentials_store"] = toml_edit::value("file");
    Ok(doc.to_string())
}
pub fn resolve_home(user: &Path, override_home: Option<&str>) -> Result<PathBuf, AuthError> {
    match override_home.filter(|p| !p.trim().is_empty()) {
        Some(p) if Path::new(p).is_absolute() => Ok(PathBuf::from(p)),
        Some(_) => Err(AuthError::Config),
        None => Ok(user.join(".codex")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn payload(access: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"userId":"test-user", "tokens":{"account_id":"test-workspace", "access_token":access,"refresh_token":"TEST_ONLY"},"future":{"keep":true}})).unwrap()
    }
    #[test]
    fn identity_survives_refresh_and_preserves_bytes() {
        let a = payload("TEST_ONLY_A");
        let parsed = AuthPayload::parse(a.clone()).unwrap();
        assert_eq!(parsed.bytes(), a);
        assert_eq!(
            parsed.identity,
            AuthPayload::parse(payload("TEST_ONLY_B")).unwrap().identity
        );
    }
    #[test]
    fn shared_workspace_is_not_user_identity() {
        assert!(AuthPayload::parse(br#"{"tokens":{"account_id":"test-workspace","access_token":"TEST_ONLY","refresh_token":"TEST_ONLY"}}"#.to_vec()).is_err());
    }
    #[test]
    fn malformed_is_rejected() {
        for s in ["null", "[]", "{}", "not json"] {
            assert!(AuthPayload::parse(s.as_bytes().to_vec()).is_err());
        }
    }
    #[test]
    fn config_preserves_other_settings() {
        let s = "# keep this\nmodel = 'example'\n[mcp_servers.local]\ncommand = 'example'\n";
        let out = file_storage_config(s).unwrap();
        assert!(out.contains("# keep this"));
        assert!(out.contains("[mcp_servers.local]"));
        assert_eq!(storage_mode(&out).unwrap(), CredentialStore::File);
    }
    #[test]
    fn home_resolution() {
        let home = std::env::temp_dir();
        assert_eq!(resolve_home(&home, None).unwrap(), home.join(".codex"));
        assert!(resolve_home(&home, Some("relative")).is_err());
        assert_eq!(resolve_home(&home, home.to_str()).unwrap(), home);
    }
}

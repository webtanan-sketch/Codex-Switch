//! Read-only quota metadata through the official Codex app-server protocol.
use crate::{system, Error, Manager, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub kind: String,
    pub used_percent: f64,
    pub duration_mins: Option<u64>,
    pub resets_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub profile_id: String,
    pub checked_at: u64,
    pub windows: Vec<UsageWindow>,
    pub ordinary_usage_allowed: Option<bool>,
}

fn parse_rate_limits(result: &Value) -> Result<Vec<UsageWindow>> {
    let bucket = result
        .get("rateLimitsByLimitId")
        .and_then(|v| v.get("codex"))
        .filter(|v| v.is_object())
        .or_else(|| result.get("rateLimits"))
        .ok_or(Error::UsageUnavailable)?;
    if bucket
        .get("limitId")
        .and_then(Value::as_str)
        .is_some_and(|id| id != "codex")
    {
        return Err(Error::UsageUnavailable);
    }
    let mut windows = Vec::new();
    for kind in ["primary", "secondary"] {
        let Some(window) = bucket.get(kind).filter(|v| !v.is_null()) else {
            continue;
        };
        let used_percent = window
            .get("usedPercent")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or(Error::UsageUnavailable)?;
        let optional_number = |key| -> Result<Option<u64>> {
            match window.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(value) => value.as_u64().map(Some).ok_or(Error::UsageUnavailable),
            }
        };
        let duration_mins = optional_number("windowDurationMins")?;
        if duration_mins == Some(0) {
            return Err(Error::UsageUnavailable);
        }
        windows.push(UsageWindow {
            kind: kind.into(),
            used_percent,
            duration_mins,
            resets_at: optional_number("resetsAt")?,
        });
    }
    if windows.is_empty() {
        return Err(Error::UsageUnavailable);
    }
    Ok(windows)
}

// Own the process even on timeout/protocol errors. Its stderr and all raw
// account responses are discarded; only allowlisted quota metadata leaves Rust.
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
    const MAX_FRAME: usize = 512 * 1024;
    let mut bytes = Vec::new();
    Read::take(&mut *reader, (MAX_FRAME + 1) as u64).read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_FRAME {
        return Err(Error::UsageUnavailable);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| Error::UsageUnavailable)
}

fn response(rx: &Receiver<Result<Value>>, id: u64, deadline: Instant) -> Result<Value> {
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .ok_or(Error::UsageTimeout)?;
        let message = rx.recv_timeout(left).map_err(|e| match e {
            mpsc::RecvTimeoutError::Timeout => Error::UsageTimeout,
            mpsc::RecvTimeoutError::Disconnected => Error::UsageUnavailable,
        })??;
        if message.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if message.get("error").is_some() {
            return Err(Error::UsageUnavailable);
        }
        return message
            .get("result")
            .cloned()
            .ok_or(Error::UsageUnavailable);
    }
}

fn fetch(exe: &Path, home: &Path) -> Result<Value> {
    system::validate_exe(exe)?;
    let mut command = Command::new(exe);
    system::clean_env(&mut command);
    command
        .env("CODEX_HOME", home)
        .current_dir(home)
        .args([
            "-c",
            "cli_auth_credentials_store=\"file\"",
            "app-server",
            "--listen",
            "stdio://",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    let mut server = Server(command.spawn().map_err(|_| Error::UsageUnavailable)?);
    let mut input = server.0.stdin.take().ok_or(Error::UsageUnavailable)?;
    let output = server.0.stdout.take().ok_or(Error::UsageUnavailable)?;
    let (tx, rx) = mpsc::sync_channel(8);
    std::thread::Builder::new()
        .name("codex-usage-reader".into())
        .spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let message = match read_message(&mut reader) {
                    Ok(Some(value)) => Ok(value),
                    Ok(None) => break,
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        break;
                    }
                };
                if tx.send(message).is_err() {
                    break;
                }
            }
        })
        .map_err(|_| Error::UsageUnavailable)?;
    let mut send = |value: Value| -> Result<()> {
        serde_json::to_writer(&mut input, &value).map_err(|_| Error::UsageUnavailable)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(())
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    send(serde_json::json!({"id":1,"method":"initialize","params":{
        "clientInfo":{"name":"codex_account_manager","title":"Codex Account Manager","version":"0.1.1"}
    }}))?;
    response(&rx, 1, deadline)?;
    send(serde_json::json!({"method":"initialized","params":{}}))?;
    send(serde_json::json!({"id":2,"method":"account/read","params":{"refreshToken":false}}))?;
    let account = response(&rx, 2, deadline)?;
    if account.pointer("/account/type").and_then(Value::as_str) != Some("chatgpt") {
        return Err(Error::UsageUnavailable);
    }
    // No thread/turn, login, logout, reset-credit redemption, or inference call.
    send(serde_json::json!({"id":3,"method":"account/rateLimits/read","params":{}}))?;
    response(&rx, 3, deadline)
}

impl Manager {
    pub fn usage_profile_id(&self) -> Result<String> {
        let auth = codex_auth::AuthPayload::parse(std::fs::read(self.home.join("auth.json"))?)?;
        self.state()?
            .profiles
            .into_iter()
            .find(|profile| profile.identity.fingerprint == auth.identity.fingerprint)
            .map(|profile| profile.id)
            .ok_or(Error::UnknownActive)
    }

    pub fn read_usage(&self, exe: &Path) -> Result<UsageSnapshot> {
        let profile_id = self.usage_profile_id()?;
        let result = fetch(exe, &self.home)?;
        if self.usage_profile_id()? != profile_id {
            return Err(Error::ConcurrentWrite);
        }
        Ok(UsageSnapshot {
            profile_id,
            checked_at: crate::now(),
            windows: parse_rate_limits(&result)?,
            ordinary_usage_allowed: result.get("ordinaryUsageAllowed").and_then(Value::as_bool),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rpc_frames_are_bounded_and_invalid_frames_fail_closed() {
        let mut reader = std::io::Cursor::new(b"{\"id\":1,\"result\":{}}\n{\"id\":2}\n");
        assert_eq!(read_message(&mut reader).unwrap().unwrap()["id"], 1);
        assert_eq!(read_message(&mut reader).unwrap().unwrap()["id"], 2);
        assert!(read_message(&mut reader).unwrap().is_none());
        assert!(read_message(&mut std::io::Cursor::new(b"not json\n")).is_err());
        assert!(read_message(&mut std::io::Cursor::new(vec![b'x'; 512 * 1024 + 1])).is_err());
    }

    #[test]
    fn rpc_response_ignores_notifications_and_does_not_expose_server_errors() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok(json!({"method":"account/updated","params":{}})))
            .unwrap();
        tx.send(Ok(json!({"id":1,"result":{}}))).unwrap();
        tx.send(Ok(json!({"id":2,"result":{"test":true}}))).unwrap();
        assert_eq!(
            response(&rx, 2, Instant::now() + Duration::from_secs(1)).unwrap()["test"],
            true
        );
        tx.send(Ok(json!({"id":3,"error":{"message":"PRIVATE_TEST_ERROR"}})))
            .unwrap();
        assert!(!response(&rx, 3, Instant::now() + Duration::from_secs(1))
            .unwrap_err()
            .to_string()
            .contains("PRIVATE_TEST"));
        assert!(matches!(
            response(&rx, 4, Instant::now()),
            Err(Error::UsageTimeout)
        ));
    }

    #[test]
    fn codex_bucket_wins_over_unrelated_legacy_limits() {
        let windows = parse_rate_limits(&json!({
            "rateLimits": {"limitId":"other","primary":{"usedPercent":90}},
            "rateLimitsByLimitId": {"codex": {
                "primary":{"usedPercent":23,"windowDurationMins":300,"resetsAt":1793100000},
                "secondary":{"usedPercent":61,"windowDurationMins":10080,"resetsAt":1793600000}
            }}
        }))
        .unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].used_percent, 23.0);
        assert_eq!(windows[0].duration_mins, Some(300));
        assert_eq!(windows[1].kind, "secondary");
        assert_eq!(windows[1].resets_at, Some(1793600000));
    }

    #[test]
    fn older_single_bucket_and_unknown_reset_are_supported() {
        let windows = parse_rate_limits(&json!({"rateLimits": {
            "primary":null,
            "secondary":{"usedPercent":100,"windowDurationMins":null,"resetsAt":null}
        }}))
        .unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].kind, "secondary");
        assert_eq!(windows[0].duration_mins, None);
        assert_eq!(windows[0].resets_at, None);
    }

    #[test]
    fn missing_or_invalid_limits_never_become_zero_usage() {
        for value in [
            json!({}),
            json!({"rateLimits":{"primary":null,"secondary":null}}),
            json!({"rateLimits":{"primary":{"usedPercent":-1}}}),
            json!({"rateLimits":{"primary":{"usedPercent":"20"}}}),
            json!({"rateLimits":{"primary":{"usedPercent":20,"windowDurationMins":-5}}}),
            json!({"rateLimits":{"primary":{"usedPercent":20,"resetsAt":-1}}}),
            json!({"rateLimits":{"limitId":"unrelated","primary":{"usedPercent":20}}}),
        ] {
            assert!(matches!(
                parse_rate_limits(&value),
                Err(Error::UsageUnavailable)
            ));
        }
    }

    #[test]
    fn quota_output_contains_only_allowlisted_metadata() {
        let windows = parse_rate_limits(&json!({"rateLimits": {
            "primary":{"usedPercent":105,"windowDurationMins":300,"resetsAt":1793100000,
                       "access_token":"TEST_ONLY"},
            "email":"PRIVATE_TEST", "refresh_token":"TEST_ONLY"
        }}))
        .unwrap();
        let output = serde_json::to_string(&windows).unwrap();
        assert!(!output.contains("TEST_ONLY"));
        assert!(!output.contains("PRIVATE_TEST"));
        assert_eq!(windows[0].used_percent, 105.0);
    }
}

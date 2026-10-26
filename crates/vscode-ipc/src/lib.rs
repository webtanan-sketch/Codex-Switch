//! One-way metadata-only broadcasts. Never accepts authentication or control commands.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
pub const PIPE: &str = r"\\.\pipe\CodexAccountManager";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileEvent {
    pub version: u8,
    #[serde(rename = "type")]
    pub kind: String,
    pub home_key: String,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub timestamp: u64,
}
pub fn home_key(path: &Path) -> String {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase();
    format!("{:x}", Sha256::digest(normalized.as_bytes()))
}
#[derive(Clone, Default)]
pub struct Broadcaster {
    latest: Arc<Mutex<Option<ProfileEvent>>>,
}
impl Broadcaster {
    pub fn publish(&self, event: ProfileEvent) {
        if let Ok(mut value) = self.latest.lock() {
            *value = Some(event);
        }
    }
    pub fn latest(&self) -> Option<ProfileEvent> {
        self.latest.lock().ok()?.clone()
    }
    pub fn start(&self) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            windows::start(self.clone())
        }
        #[cfg(not(windows))]
        {
            Err(std::io::ErrorKind::Unsupported.into())
        }
    }
}
#[cfg(windows)]
mod windows {
    use super::*;
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, time::Duration};
    use windows_sys::Win32::{
        Foundation::*,
        Security::*,
        Storage::FileSystem::*,
        System::{Pipes::*, Threading::*, IO::*},
    };
    // Restrict clients to the current logon user by reusing an owner-only private
    // file's security descriptor. Named pipe remote clients are explicitly rejected.
    pub fn start(bus: Broadcaster) -> std::io::Result<()> {
        let root = std::env::var_os("APPDATA").ok_or(std::io::ErrorKind::NotFound)?;
        let dir = std::path::PathBuf::from(root).join("CodexAccountManager");
        profile_vault::private_dir(&dir).map_err(|_| std::io::ErrorKind::PermissionDenied)?;
        let file: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut size = 0;
        unsafe {
            GetFileSecurityW(
                file.as_ptr(),
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                0,
                &mut size,
            );
        }
        if size == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut descriptor = vec![0u64; (size as usize).div_ceil(8)];
        if unsafe {
            GetFileSecurityW(
                file.as_ptr(),
                DACL_SECURITY_INFORMATION,
                descriptor.as_mut_ptr() as _,
                size,
                &mut size,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let name: Vec<u16> = OsStr::new(PIPE).encode_wide().chain(Some(0)).collect();
        let mut make = move |first: bool| -> std::io::Result<HANDLE> {
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.as_mut_ptr() as _,
                bInheritHandle: 0,
            };
            let handle = unsafe {
                CreateNamedPipeW(
                    name.as_ptr(),
                    PIPE_ACCESS_OUTBOUND
                        | FILE_FLAG_OVERLAPPED
                        | if first {
                            FILE_FLAG_FIRST_PIPE_INSTANCE
                        } else {
                            0
                        },
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    32,
                    8192,
                    0,
                    0,
                    &attributes,
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(handle)
            }
        };
        // First-instance flag prevents connecting companions to a pre-existing server.
        let first = make(true)? as usize;
        std::thread::spawn(move || {
            let mut pending = Some(first);
            loop {
                let raw = match pending.take() {
                    Some(h) => h as HANDLE,
                    None => match make(false) {
                        Ok(h) => h,
                        Err(_) => {
                            std::thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    },
                };
                unsafe {
                    let event = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
                    let mut ov: OVERLAPPED = std::mem::zeroed();
                    ov.hEvent = event;
                    let result = ConnectNamedPipe(raw, &mut ov);
                    let error = GetLastError();
                    if result == 0 && error == ERROR_IO_PENDING {
                        WaitForSingleObject(event, INFINITE);
                    } else if result == 0 && error != ERROR_PIPE_CONNECTED {
                        CloseHandle(event);
                        CloseHandle(raw);
                        continue;
                    }
                    CloseHandle(event);
                }
                let peer_bus = bus.clone();
                let handle = raw as usize;
                // Each client has its own writer thread. A stalled client cannot block switching.
                std::thread::spawn(move || {
                    let raw = handle as HANDLE;
                    let mut last = String::new();
                    let mut last_sent = std::time::Instant::now();
                    loop {
                        if let Some(event) = peer_bus.latest() {
                            let mut line = match serde_json::to_vec(&event) {
                                Ok(v) => v,
                                Err(_) => break,
                            };
                            line.push(b'\n');
                            let current = String::from_utf8_lossy(&line).to_string();
                            if current != last || last_sent.elapsed() > Duration::from_secs(10) {
                                // Overlapped write with a bounded deadline to prevent stuck clients.
                                unsafe {
                                    let signal =
                                        CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
                                    let mut ov: OVERLAPPED = std::mem::zeroed();
                                    ov.hEvent = signal;
                                    let mut count = 0;
                                    let result = WriteFile(
                                        raw,
                                        line.as_ptr(),
                                        line.len() as u32,
                                        &mut count,
                                        &mut ov,
                                    );
                                    let mut ok = result != 0;
                                    if !ok && GetLastError() == ERROR_IO_PENDING {
                                        ok = WaitForSingleObject(signal, 2000) == WAIT_OBJECT_0
                                            && GetOverlappedResult(raw, &ov, &mut count, 0) != 0;
                                        if !ok {
                                            CancelIoEx(raw, &ov);
                                            WaitForSingleObject(signal, INFINITE);
                                        }
                                    }
                                    CloseHandle(signal);
                                    if !ok {
                                        break;
                                    }
                                }
                                last = current;
                                last_sent = std::time::Instant::now();
                            }
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    unsafe {
                        DisconnectNamedPipe(raw);
                        CloseHandle(raw);
                    }
                });
            }
        });
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_only_protocol() {
        let event = ProfileEvent {
            version: 1,
            kind: "PROFILE_CHANGED".into(),
            home_key: "test-home".into(),
            profile_id: Some("test-id".into()),
            profile_name: Some("PlayFood".into()),
            timestamp: 1,
        };
        let bytes = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<ProfileEvent>(&bytes).unwrap(), event);
        assert!(!bytes.contains("token"));
        assert!(serde_json::from_str::<ProfileEvent>(
            &bytes.replace("\"version\":1", "\"version\":1,\"auth\":{} ")
        )
        .is_err());
    }
}

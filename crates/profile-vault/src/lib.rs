//! Per-user encrypted storage. Non-Windows production builds fail closed.
use std::{fs, io::Write, path::Path};
use zeroize::Zeroizing;
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Secure storage operation failed. Check Windows account and file permissions.")]
    Storage,
    #[error("DPAPI is only available on Windows.")]
    Unsupported,
    #[error("Windows credential encryption failed (OS code {0}).")]
    Encryption(u32),
    #[error("Windows credential decryption failed (OS code {0}).")]
    Decryption(u32),
    #[error("Atomic file replacement failed (OS code {0:?}).")]
    Replace(Option<i32>),
}
pub trait Protector: Send + Sync {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, VaultError>;
    fn unprotect(&self, encrypted: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultError>;
}
pub struct Dpapi;
#[cfg(not(windows))]
impl Protector for Dpapi {
    fn protect(&self, _: &[u8]) -> Result<Vec<u8>, VaultError> {
        Err(VaultError::Unsupported)
    }
    fn unprotect(&self, _: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        Err(VaultError::Unsupported)
    }
}
#[cfg(windows)]
impl Protector for Dpapi {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, VaultError> {
        dpapi(plain, false).map(|b| b.to_vec())
    }
    fn unprotect(&self, encrypted: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        dpapi(encrypted, true)
    }
}
#[cfg(windows)]
fn dpapi(input: &[u8], decrypt: bool) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    if input.is_empty() || input.len() > u32::MAX as usize {
        return Err(VaultError::Storage);
    }
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut out: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        if decrypt {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        } else {
            CryptProtectData(
                &source,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        }
    };
    if ok == 0 {
        let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        return Err(if decrypt {
            VaultError::Decryption(code)
        } else {
            VaultError::Encryption(code)
        });
    }
    let result = Zeroizing::new(
        unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec(),
    );
    unsafe {
        std::ptr::write_bytes(out.pbData, 0, out.cbData as usize);
        LocalFree(out.pbData as _);
    }
    Ok(result)
}
// Owner + SYSTEM only; inherited permissions are removed before secret files exist.
#[cfg(windows)]
pub fn private_dir(path: &Path) -> Result<(), VaultError> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree},
        Security::Authorization::*,
        Security::*,
        System::Threading::*,
    };
    fs::create_dir_all(path).map_err(|_| VaultError::Storage)?;
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(VaultError::Storage);
        }
        let mut len = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buffer = vec![0u64; (len as usize).div_ceil(8)];
        let ok = GetTokenInformation(token, TokenUser, buffer.as_mut_ptr() as _, len, &mut len);
        CloseHandle(token);
        if ok == 0 {
            return Err(VaultError::Storage);
        }
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(VaultError::Storage);
        }
        let mut n = 0;
        while *sid.add(n) != 0 {
            n += 1;
        }
        let sid_string = String::from_utf16_lossy(std::slice::from_raw_parts(sid, n));
        LocalFree(sid as _);
        let sddl: Vec<u16> = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid_string})\0")
            .encode_utf16()
            .collect();
        let mut descriptor = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(VaultError::Storage);
        }
        use std::os::windows::ffi::OsStrExt;
        let p: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = SetFileSecurityW(
            p.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        );
        LocalFree(descriptor as _);
        if ok == 0 {
            return Err(VaultError::Storage);
        }
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn private_dir(path: &Path) -> Result<(), VaultError> {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path).map_err(|_| VaultError::Storage)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| VaultError::Storage)
}
fn prepare_atomic_write(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, VaultError> {
    let dir = path.parent().ok_or(VaultError::Storage)?;
    fs::create_dir_all(dir).map_err(|_| VaultError::Storage)?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".cam-")
        .tempfile_in(dir)
        .map_err(|_| VaultError::Storage)?;
    tmp.write_all(bytes)
        .and_then(|_| tmp.as_file().sync_all())
        .map_err(|_| VaultError::Storage)?;
    Ok(tmp)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let tmp = prepare_atomic_write(path, bytes)?;
    // persist uses MoveFileExW(REPLACE_EXISTING) on Windows; never delete-then-rename.
    tmp.persist(path)
        .map_err(|e| VaultError::Replace(e.error.raw_os_error()))?;
    #[cfg(unix)]
    fs::File::open(path.parent().ok_or(VaultError::Storage)?)
        .and_then(|d| d.sync_all())
        .map_err(|_| VaultError::Storage)?;
    Ok(())
}
pub fn save(path: &Path, bytes: &[u8], protector: &dyn Protector) -> Result<(), VaultError> {
    atomic_write(path, &protector.protect(bytes)?)
}
pub fn load(path: &Path, protector: &dyn Protector) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    protector.unprotect(&fs::read(path).map_err(|_| VaultError::Storage)?)
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn dpapi_roundtrip_and_tampering() {
        let p = b"TEST_ONLY_PLAINTEXT";
        let mut encrypted = Dpapi.protect(p).unwrap();
        assert!(!encrypted.windows(p.len()).any(|w| w == p));
        assert_eq!(&*Dpapi.unprotect(&encrypted).unwrap(), p);
        let i = encrypted.len() / 2;
        encrypted[i] ^= 0xff;
        assert!(Dpapi.unprotect(&encrypted).is_err());
    }
}
#[cfg(test)]
mod atomic_tests {
    use super::*;
    #[test]
    fn failure_after_flush_before_replace_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("active");
        atomic_write(&path, b"TEST_ONLY_OLD").unwrap();
        let prepared = prepare_atomic_write(&path, b"TEST_ONLY_NEW").unwrap();
        assert_eq!(fs::read(prepared.path()).unwrap(), b"TEST_ONLY_NEW");
        assert_eq!(fs::read(&path).unwrap(), b"TEST_ONLY_OLD");
        drop(prepared);
        assert_eq!(fs::read(&path).unwrap(), b"TEST_ONLY_OLD");
    }
    #[test]
    fn replacement_is_complete_and_temp_files_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("active");
        atomic_write(&path, b"TEST_ONLY_OLD").unwrap();
        atomic_write(&path, b"TEST_ONLY_NEW").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"TEST_ONLY_NEW");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}

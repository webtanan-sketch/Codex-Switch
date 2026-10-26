use account_core::{redact, Error, Fault, Manager};
use profile_vault::{Dpapi, Protector, VaultError};
use std::{collections::HashMap, fs, sync::Arc};
use zeroize::Zeroizing;
// Non-Windows logic tests use a test-only reversible protector. Shipping code
// never uses it; the same tests use real Windows DPAPI on Windows CI.
struct TestProtector;
impl Protector for TestProtector {
    fn protect(&self, p: &[u8]) -> Result<Vec<u8>, VaultError> {
        let mut out = b"TEST_ONLY".to_vec();
        out.extend(p.iter().rev().map(|x| x ^ 0x5a));
        Ok(out)
    }
    fn unprotect(&self, p: &[u8]) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let p = p.strip_prefix(b"TEST_ONLY").ok_or(VaultError::Storage)?;
        Ok(Zeroizing::new(p.iter().rev().map(|x| x ^ 0x5a).collect()))
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    manager: Manager,
}
fn auth(user: &str, version: u32) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"userId": user,"tokens":{"account_id":"TEST_WORKSPACE","access_token":format!("TEST_ONLY_{version}"),"refresh_token":"TEST_ONLY"}, "unknown":{"preserve":[1,2,3]}})).unwrap()
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let p: Arc<dyn Protector> = if cfg!(windows) {
            Arc::new(Dpapi)
        } else {
            Arc::new(TestProtector)
        };
        let manager = Manager::new(
            dir.path().join("app"),
            dir.path().join("codex"),
            dir.path().join("hub"),
            p,
        )
        .unwrap();
        manager.configure_file_storage().unwrap();
        Self { _dir: dir, manager }
    }
    fn write(&self, user: &str, version: u32) {
        fs::write(self.manager.home.join("auth.json"), auth(user, version)).unwrap();
    }
    fn read(&self) -> Vec<u8> {
        fs::read(self.manager.home.join("auth.json")).unwrap()
    }
    fn pair(&self) -> (String, String) {
        self.write("TEST_A", 1);
        let a = self.manager.import_current("Webtanan").unwrap().id;
        self.write("TEST_B", 1);
        let b = self.manager.import_current("PlayFood").unwrap().id;
        self.manager.switch(&a).unwrap();
        (a, b)
    }
}
#[test]
fn real_switch_preserves_unknown_bytes() {
    let f = Fixture::new();
    let (_, b) = f.pair();
    f.manager.switch(&b).unwrap();
    assert_eq!(f.read(), auth("TEST_B", 1));
    assert_eq!(f.manager.state().unwrap().active_profile_id, Some(b));
}
#[test]
fn refreshed_credentials_are_restored_on_later_switch() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    f.manager.switch(&b).unwrap();
    f.write("TEST_B", 2);
    assert!(f.manager.sync_files(&mut HashMap::new()).unwrap());
    f.manager.switch(&a).unwrap();
    f.manager.switch(&b).unwrap();
    assert_eq!(f.read(), auth("TEST_B", 2));
}
#[test]
fn watcher_does_not_rewrite_unchanged_profile() {
    let f = Fixture::new();
    f.pair();
    let mut observed = HashMap::new();
    assert!(f.manager.sync_files(&mut observed).unwrap());
    assert!(!f.manager.sync_files(&mut observed).unwrap());
}
#[test]
fn watcher_does_not_capture_a_different_user() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    f.write("TEST_UNKNOWN", 2);
    assert!(!f.manager.sync_files(&mut HashMap::new()).unwrap());
    assert!(matches!(f.manager.switch(&b), Err(Error::UnknownActive)));
    f.write("TEST_A", 1);
    f.manager.switch(&a).unwrap();
}
#[test]
fn before_replace_leaves_original_intact_and_recovers() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    let before = f.read();
    assert!(f
        .manager
        .switch_with_fault(&b, Fault::BeforeReplace)
        .is_err());
    assert_eq!(f.read(), before);
    f.manager.recover_startup().unwrap();
    assert_eq!(f.manager.state().unwrap().active_profile_id, Some(a));
}
#[test]
fn after_replace_is_complete_and_recovers_forward() {
    let f = Fixture::new();
    let (_, b) = f.pair();
    assert!(f
        .manager
        .switch_with_fault(&b, Fault::AfterReplace)
        .is_err());
    assert_eq!(f.read(), auth("TEST_B", 1));
    f.manager.recover_startup().unwrap();
    assert_eq!(f.manager.state().unwrap().active_profile_id, Some(b));
}
#[test]
fn verification_failure_rolls_back() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    assert!(matches!(
        f.manager.switch_with_fault(&b, Fault::Verification),
        Err(Error::Verification)
    ));
    assert_eq!(f.read(), auth("TEST_A", 1));
    assert_eq!(f.manager.state().unwrap().active_profile_id, Some(a));
}
#[test]
fn corrupt_target_cannot_replace_active_auth() {
    let f = Fixture::new();
    let (_, b) = f.pair();
    let before = f.read();
    fs::write(
        f.manager.root.join("vault").join(format!("{b}.bin")),
        b"CORRUPT_TEST_ONLY",
    )
    .unwrap();
    assert!(f.manager.switch(&b).is_err());
    assert_eq!(f.read(), before);
}
#[test]
fn missing_auth_is_handled() {
    let f = Fixture::new();
    assert!(f.manager.import_current("Account").is_err());
    let (_, b) = f.pair();
    fs::remove_file(f.manager.home.join("auth.json")).unwrap();
    f.manager.switch(&b).unwrap();
    assert_eq!(f.read(), auth("TEST_B", 1));
}
#[test]
fn invalid_auth_is_never_overwritten_by_switch() {
    let f = Fixture::new();
    let (_, b) = f.pair();
    fs::write(f.manager.home.join("auth.json"), b"INVALID_TEST_ONLY").unwrap();
    assert!(f.manager.switch(&b).is_err());
    assert_eq!(f.read(), b"INVALID_TEST_ONLY");
}
#[test]
fn isolated_homes_do_not_change_global_auth() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    let before = f.read();
    let one = f.manager.prepare_isolated(&a).unwrap();
    let two = f.manager.prepare_isolated(&b).unwrap();
    assert_ne!(one, two);
    assert_eq!(f.read(), before);
    assert_eq!(fs::read(two.join("auth.json")).unwrap(), auth("TEST_B", 1));
}
#[test]
fn login_isolated_and_cleanup_follows_save() {
    let f = Fixture::new();
    f.pair();
    let before = f.read();
    let (id, home) = f.manager.prepare_login().unwrap();
    assert_eq!(f.read(), before);
    assert!(matches!(
        f.manager.finish_login(&id, "New"),
        Err(Error::LoginPending)
    ));
    fs::write(home.join("auth.json"), auth("TEST_NEW", 1)).unwrap();
    f.manager.finish_login(&id, "New").unwrap();
    assert!(!home.exists());
    assert_eq!(f.read(), before);
}
#[test]
fn lock_prevents_parallel_switches() {
    use fs2::FileExt;
    let f = Fixture::new();
    let (_, b) = f.pair();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.manager.root.join("switch.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    assert!(matches!(f.manager.switch(&b), Err(Error::Busy)));
    FileExt::unlock(&lock).unwrap();
}
#[test]
fn pending_journal_blocks_a_second_transaction() {
    let f = Fixture::new();
    let (a, b) = f.pair();
    let _ = f.manager.switch_with_fault(&b, Fault::BeforeReplace);
    assert!(matches!(f.manager.switch(&a), Err(Error::Recovery)));
}
#[test]
fn storage_mode_requires_explicit_opt_in_and_keeps_config() {
    let f = Fixture::new();
    let (_, b) = f.pair();
    let config = "# TEST_COMMENT\ncli_auth_credentials_store = 'keyring'\nmodel = 'example'\n";
    fs::write(f.manager.home.join("config.toml"), config).unwrap();
    assert!(matches!(f.manager.switch(&b), Err(Error::StorageMode)));
    f.manager.configure_file_storage().unwrap();
    let new = fs::read_to_string(f.manager.home.join("config.toml")).unwrap();
    assert!(new.contains("# TEST_COMMENT"));
    assert!(new.contains("model = 'example'"));
    assert!(fs::read_dir(&f.manager.home).unwrap().flatten().any(|x| x
        .file_name()
        .to_string_lossy()
        .starts_with("config.cam-backup-")));
}
#[test]
fn names_and_profile_paths_are_validated() {
    let f = Fixture::new();
    f.write("TEST_A", 1);
    assert!(matches!(f.manager.import_current("\n"), Err(Error::Name)));
    assert!(f.manager.profile_home("../../outside").is_err());
}
#[test]
fn reset_keeps_profiles_vault_and_codex_login() {
    let f = Fixture::new();
    f.pair();
    let before = f.read();
    f.manager.reset_state().unwrap();
    assert_eq!(f.read(), before);
    assert_eq!(f.manager.state().unwrap().profiles.len(), 2);
    assert!(f.manager.state().unwrap().active_profile_id.is_none());
}
#[test]
fn redaction_and_audit_never_emit_credentials() {
    let f = Fixture::new();
    f.pair();
    account_core::audit(&f.manager.root, "access_token TEST_ONLY_SECRET", false);
    let log = fs::read_to_string(f.manager.root.join("app.log")).unwrap();
    assert!(!log.contains("TEST_ONLY"));
    assert!(redact("Authorization: Bearer TEST_ONLY_SECRET").contains("REDACTED"));
    assert!(!redact("access_token=TEST_ONLY_SECRET").contains("TEST_ONLY_SECRET"));
    assert!(!redact("oauth_TEST_ONLY_SECRET").contains("TEST_ONLY_SECRET"));
}
#[test]
fn repair_never_overwrites_unknown_login() {
    let f = Fixture::new();
    f.pair();
    f.write("TEST_UNKNOWN", 1);
    assert!(matches!(
        f.manager.repair_active(),
        Err(Error::UnknownActive)
    ));
    assert_eq!(f.read(), auth("TEST_UNKNOWN", 1));
}
#[test]
fn repair_restores_a_corrupt_active_file() {
    let f = Fixture::new();
    f.pair();
    fs::write(f.manager.home.join("auth.json"), b"INVALID_TEST_ONLY").unwrap();
    f.manager.repair_active().unwrap();
    assert_eq!(f.read(), auth("TEST_A", 1));
}
#[test]
fn ordinary_json_is_not_a_legacy_profile_migration() {
    let f = Fixture::new();
    let user = f._dir.path().join("user");
    fs::create_dir_all(user.join(".codex-switch/maintenance/v1/cache/profiles")).unwrap();
    let report = account_core::system::diagnostics(&f.manager, &user).unwrap();
    assert!(report.legacy_maintenance_present);
    assert!(!report.legacy_profiles_present);
}
#[test]
fn unsupported_platform_vault_fails_closed() {
    if !cfg!(windows) {
        assert!(Dpapi.protect(b"TEST_ONLY").is_err());
    }
}

#[test]
fn selecting_active_profile_never_restores_an_older_vault_copy() {
    let f = Fixture::new();
    let (a, _) = f.pair();
    f.write("TEST_A", 99);
    f.manager.switch(&a).unwrap();
    assert_eq!(f.read(), auth("TEST_A", 99));
}

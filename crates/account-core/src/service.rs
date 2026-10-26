use crate::*;
use codex_auth::{AuthPayload, CredentialStore};
use fs2::FileExt;
use profile_vault::{atomic_write, load, private_dir, save, Protector};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct Manager {
    pub root: PathBuf,
    pub home: PathBuf,
    pub hub: PathBuf,
    protector: Arc<dyn Protector>,
}
struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Journal {
    target: String,
    previous: Option<String>,
    had_auth: bool,
}
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Fault {
    #[default]
    None,
    BeforeReplace,
    AfterReplace,
    Verification,
}
fn auth_read(path: &Path) -> Result<AuthPayload> {
    AuthPayload::parse(fs::read(path)?).map_err(Into::into)
}
fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok()
}
impl Manager {
    pub fn new(
        root: PathBuf,
        home: PathBuf,
        hub: PathBuf,
        protector: Arc<dyn Protector>,
    ) -> Result<Self> {
        private_dir(&root)?;
        private_dir(&root.join("vault"))?;
        private_dir(&hub)?;
        Ok(Self {
            root,
            home,
            hub,
            protector,
        })
    }
    fn lock(&self) -> Result<Lock> {
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("switch.lock"))?;
        f.try_lock_exclusive().map_err(|_| Error::Busy)?;
        Ok(Lock(f))
    }
    fn vault(&self, id: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            return Err(Error::Profile);
        }
        Ok(self.root.join("vault").join(format!("{id}.bin")))
    }
    pub fn state(&self) -> Result<State> {
        json_read(&self.root.join("state.json"))
    }
    pub fn settings(&self) -> Result<Settings> {
        json_read(&self.root.join("settings.json"))
    }
    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        let _lock = self.lock()?;
        for s in [&settings.vscode_path, &settings.codex_path]
            .into_iter()
            .flatten()
        {
            system::validate_exe(Path::new(s))?;
        }
        json_write(&self.root.join("settings.json"), &settings)
    }
    fn write_state(&self, state: &State) -> Result<()> {
        json_write(&self.root.join("state.json"), state)
    }
    fn require_no_journal(&self) -> Result<()> {
        if self.root.join("transaction.json").exists() {
            Err(Error::Recovery)
        } else {
            Ok(())
        }
    }
    fn payload(&self, id: &str, state: &State) -> Result<AuthPayload> {
        let p = state
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::Profile)?;
        let auth = AuthPayload::parse(load(&self.vault(id)?, self.protector.as_ref())?.to_vec())?;
        if auth.identity.fingerprint != p.identity.fingerprint {
            return Err(Error::Profile);
        }
        Ok(auth)
    }
    fn sync_payload(&self, auth: &AuthPayload, state: &mut State) -> Result<String> {
        let p = state
            .profiles
            .iter_mut()
            .find(|p| p.identity.fingerprint == auth.identity.fingerprint)
            .ok_or(Error::UnknownActive)?;
        save(&self.vault(&p.id)?, auth.bytes(), self.protector.as_ref())?;
        p.last_synced_at = now();
        Ok(p.id.clone())
    }
    pub fn import_current(&self, name: &str) -> Result<Profile> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        let auth = auth_read(&self.home.join("auth.json"))?;
        let mut state = self.state()?;
        let profile = self.import_payload(name, auth, &mut state)?;
        state.active_profile_id = Some(profile.id.clone());
        self.write_state(&state)?;
        audit(&self.root, "import", true);
        Ok(profile)
    }
    fn import_payload(&self, name: &str, auth: AuthPayload, state: &mut State) -> Result<Profile> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(Error::Name);
        }
        if let Some(existing) = state
            .profiles
            .iter()
            .find(|p| p.identity.fingerprint == auth.identity.fingerprint)
            .cloned()
        {
            self.sync_payload(&auth, state)?;
            return Ok(existing);
        }
        let profile = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            identity: auth.identity.clone(),
            created_at: now(),
            last_used_at: None,
            last_synced_at: now(),
        };
        save(
            &self.vault(&profile.id)?,
            auth.bytes(),
            self.protector.as_ref(),
        )?;
        state.profiles.push(profile.clone());
        Ok(profile)
    }
    pub fn configure_file_storage(&self) -> Result<()> {
        let _lock = self.lock()?;
        fs::create_dir_all(&self.home)?;
        let path = self.home.join("config.toml");
        let config = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(_) => return Err(Error::Io),
        };
        let output = codex_auth::file_storage_config(&config)?;
        if path.exists() {
            atomic_write(
                &self
                    .home
                    .join(format!("config.cam-backup-{}.toml", uuid::Uuid::new_v4())),
                config.as_bytes(),
            )?;
        }
        atomic_write(&path, output.as_bytes())?;
        Ok(())
    }
    pub fn storage(&self) -> Result<CredentialStore> {
        let config = match fs::read_to_string(self.home.join("config.toml")) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(_) => return Err(Error::Io),
        };
        Ok(codex_auth::storage_mode(&config)?)
    }
    pub fn switch(&self, id: &str) -> Result<Profile> {
        self.switch_with_fault(id, Fault::None)
    }
    // Failure points are exposed for transaction tests, never through a desktop command.
    pub fn switch_with_fault(&self, id: &str, fault: Fault) -> Result<Profile> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        if self.storage()? != CredentialStore::File {
            return Err(Error::StorageMode);
        }
        let mut state = self.state()?;
        let target = self.payload(id, &state)?;
        let path = self.home.join("auth.json");
        let previous = match fs::read(&path) {
            Ok(b) => Some(AuthPayload::parse(b)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(Error::Io),
        };
        // Deny concurrent writes to the old inode on Windows, permit atomic replacement.
        let _auth_guard = system::guard_auth(&path)?;
        let previous_id = if let Some(ref old) = previous {
            let current = auth_read(&path)?;
            if current.digest() != old.digest() {
                return Err(Error::ConcurrentWrite);
            }
            Some(self.sync_payload(old, &mut state)?)
        } else {
            None
        };
        self.write_state(&state)?;
        if previous_id.as_deref() == Some(id) {
            state.active_profile_id = Some(id.into());
            self.write_state(&state)?;
            return state
                .profiles
                .iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or(Error::Profile);
        }
        if let Some(ref old) = previous {
            save(
                &self.root.join("rollback.bin"),
                old.bytes(),
                self.protector.as_ref(),
            )?;
        }
        let journal = Journal {
            target: id.into(),
            previous: previous_id.clone(),
            had_auth: previous.is_some(),
        };
        json_write(&self.root.join("transaction.json"), &journal)?;
        if fault == Fault::BeforeReplace {
            return Err(Error::Recovery);
        }
        // Compare immediately before commit as well; external writers don't use our lock.
        if let Some(ref old) = previous {
            if auth_read(&path)?.digest() != old.digest() {
                self.finish_journal()?;
                return Err(Error::ConcurrentWrite);
            }
        }
        if let Err(e) = atomic_write(&path, target.bytes()) {
            self.rollback(&journal)?;
            return Err(e.into());
        }
        if fault == Fault::AfterReplace {
            return Err(Error::Recovery);
        }
        let verified = auth_read(&path).is_ok_and(|p| {
            p.identity.fingerprint == target.identity.fingerprint && p.digest() == target.digest()
        });
        if !verified || fault == Fault::Verification {
            self.rollback(&journal)?;
            return Err(Error::Verification);
        }
        state.previous_profile_id = previous_id;
        state.active_profile_id = Some(id.into());
        let p = state
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Error::Profile)?;
        p.last_used_at = Some(now());
        let selected = p.clone();
        self.write_state(&state)?;
        self.finish_journal()?;
        audit(&self.root, "switch", true);
        Ok(selected)
    }
    fn finish_journal(&self) -> Result<()> {
        let path = self.root.join("transaction.json");
        if path.exists() {
            fs::remove_file(path)?;
        }
        // Keep one DPAPI-encrypted rollback for explicit repair/recovery.
        Ok(())
    }
    fn rollback(&self, journal: &Journal) -> Result<()> {
        if journal.had_auth {
            let bytes = load(&self.root.join("rollback.bin"), self.protector.as_ref())?;
            let old = AuthPayload::parse(bytes.to_vec())?;
            atomic_write(&self.home.join("auth.json"), old.bytes())?;
            if auth_read(&self.home.join("auth.json"))?.digest() != old.digest() {
                return Err(Error::Recovery);
            }
        }
        let mut state = self.state()?;
        state.active_profile_id = journal.previous.clone();
        self.write_state(&state)?;
        self.finish_journal()?;
        Ok(())
    }
    pub fn recover_startup(&self) -> Result<()> {
        let _lock = self.lock()?;
        let path = self.root.join("transaction.json");
        if !path.exists() {
            return Ok(());
        }
        let journal: Journal =
            serde_json::from_slice(&fs::read(path)?).map_err(|_| Error::Recovery)?;
        let mut state = self.state()?;
        let selected = self.payload(&journal.target, &state)?;
        if auth_read(&self.home.join("auth.json"))
            .is_ok_and(|a| a.identity.fingerprint == selected.identity.fingerprint)
        {
            state.active_profile_id = Some(journal.target);
            state.previous_profile_id = journal.previous;
            self.write_state(&state)?;
            self.finish_journal()?;
        } else {
            self.rollback(&journal)?;
        }
        audit(&self.root, "recovery", true);
        Ok(())
    }
    pub fn repair_active(&self) -> Result<()> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        let state = self.state()?;
        let id = state.active_profile_id.as_ref().ok_or(Error::Profile)?;
        // Never overwrite an unknown valid login during repair.
        let path = self.home.join("auth.json");
        if let Ok(current) = auth_read(&path) {
            if current.identity.fingerprint != self.payload(id, &state)?.identity.fingerprint {
                return Err(Error::UnknownActive);
            }
            return Ok(());
        }
        let payload = self.payload(id, &state)?;
        atomic_write(&path, payload.bytes())?;
        Ok(())
    }
    pub fn profile_home(&self, id: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            return Err(Error::Profile);
        }
        Ok(self.hub.join("homes").join(id))
    }
    pub fn prepare_isolated(&self, id: &str) -> Result<PathBuf> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        let mut state = self.state()?;
        let home = self.profile_home(id)?;
        private_dir(&home)?;
        let path = home.join("auth.json");
        let saved = self.payload(id, &state)?;
        if path.exists() {
            let active = auth_read(&path)?;
            if active.identity.fingerprint != saved.identity.fingerprint {
                return Err(Error::UnknownActive);
            }
            self.sync_payload(&active, &mut state)?;
            self.write_state(&state)?;
        } else {
            atomic_write(&path, saved.bytes())?;
        }
        let config_path = home.join("config.toml");
        let config = if config_path.exists() {
            fs::read_to_string(&config_path)?
        } else {
            String::new()
        };
        atomic_write(
            &config_path,
            codex_auth::file_storage_config(&config)?.as_bytes(),
        )?;
        Ok(home)
    }
    pub fn prepare_login(&self) -> Result<(String, PathBuf)> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        // Sync current credential without requiring a registered account for first run.
        if let Ok(auth) = auth_read(&self.home.join("auth.json")) {
            let mut state = self.state()?;
            if self.sync_payload(&auth, &mut state).is_ok() {
                self.write_state(&state)?;
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let home = self.root.join("logins").join(&id);
        private_dir(&home)?;
        atomic_write(
            &home.join("config.toml"),
            b"cli_auth_credentials_store = \"file\"\n",
        )?;
        Ok((id, home))
    }
    pub fn finish_login(&self, id: &str, name: &str) -> Result<Profile> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        if !valid_id(id) {
            return Err(Error::Profile);
        }
        let home = self.root.join("logins").join(id);
        let auth = auth_read(&home.join("auth.json")).map_err(|_| Error::LoginPending)?;
        let mut state = self.state()?;
        let p = self.import_payload(name, auth, &mut state)?;
        self.write_state(&state)?;
        // Delete only after encrypted profile and metadata are durable.
        fs::remove_dir_all(home)?;
        audit(&self.root, "login", true);
        Ok(p)
    }
    pub fn reset_state(&self) -> Result<()> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        let source = self.root.join("state.json");
        if source.exists() {
            fs::copy(
                &source,
                self.root
                    .join(format!("state.backup-{}.json", uuid::Uuid::new_v4())),
            )?;
        }
        let mut state = self.state()?;
        state.active_profile_id = None;
        state.previous_profile_id = None;
        self.write_state(&state)
    }
    /// Stable-file polling watcher (two identical reads, not platform-specific event semantics).
    /// `observed` advances only after successful sync, so errors are retried.
    pub fn sync_files(&self, observed: &mut HashMap<PathBuf, String>) -> Result<bool> {
        let _lock = self.lock()?;
        self.require_no_journal()?;
        let mut state = self.state()?;
        let mut changed = false;
        let mut sources = vec![(self.home.join("auth.json"), state.active_profile_id.clone())];
        sources.extend(
            state
                .profiles
                .iter()
                .map(|p| {
                    Ok((
                        self.profile_home(&p.id)?.join("auth.json"),
                        Some(p.id.clone()),
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        );
        for (path, expected) in sources {
            if !path.exists() {
                continue;
            }
            let auth = auth_read(&path)?;
            let digest = auth.digest();
            if observed.get(&path) == Some(&digest) {
                continue;
            }
            // Require a stable snapshot without exposing bytes to a background log.
            std::thread::sleep(std::time::Duration::from_millis(80));
            if auth_read(&path)?.digest() != digest {
                continue;
            }
            let expected = expected.and_then(|id| state.profiles.iter().find(|p| p.id == id));
            if expected.is_some_and(|p| p.identity.fingerprint == auth.identity.fingerprint) {
                self.sync_payload(&auth, &mut state)?;
                self.write_state(&state)?;
                changed = true;
            }
            observed.insert(path, digest);
        }
        Ok(changed)
    }
}

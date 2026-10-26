# Recovery

Never paste or upload `auth.json`, vault blobs, rollback blobs, login staging directories, or account tokens. Do not remove the vault to fix a UI issue.

## Interrupted switch

Start the manager. It takes the operation lock, reads `transaction.json`, and inspects the complete active payload:

- If the target identity is present, it completes metadata commit.
- Otherwise, if a previous valid login was preserved, it restores `rollback.bin` and verifies the restored bytes.
- If recovery cannot validate available files, it keeps the journal and blocks new switches. Use the Recovery screen or preserve local files before investigating.

A verification failure in a running transaction restores the previous valid credential when one exists. If there was no previous file, the manager preserves any complete newly written login rather than deleting the only valid authentication.

## Recovery controls

| Action | Behavior |
| --- | --- |
| Restore previous active account | Run the normal checked switch transaction using the previous saved profile |
| Recover interrupted transaction | Retry journal recovery |
| Repair active auth file | Restore the indexed active profile if the auth file is missing or invalid; never overwrite a different valid user's login |
| Re-import current account | Read the current active payload and save it securely under a friendly name |
| Reset application state | Back up metadata and clear active/previous pointers; retain profiles, vault, and Codex login |

If an unknown valid account is active, import it first. If the encrypted target is corrupt, reauthenticate that account through the normal Add Account flow. Do not reconstruct token fields manually.

## Configuration backup

Explicitly enabling file storage creates a uniquely named `config.cam-backup-<uuid>.toml` next to the original config before atomic replacement. It preserves other TOML values and comments where practical. Restore the appropriate backup only after exiting Codex and reviewing the change. Managed organization policies may still override it.

## Moving computers or reinstalling Windows

DPAPI protection is tied to the Windows user. Copying `.bin` files is not a supported transfer mechanism and does not guarantee recoverability. Sign in normally on the new installation. There is no portable plaintext credential export.

## Uninstalling

The NSIS installer has not yet passed upgrade/uninstall retention acceptance. Before removing development builds, understand that active global and isolated Codex homes are separate from the installed executable. Do not delete authentication or data directories as part of routine troubleshooting. Preserve your Windows account's encrypted local data if investigating a bug; never upload it.

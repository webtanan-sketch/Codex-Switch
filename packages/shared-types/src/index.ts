/** Public metadata only. Never add an authentication payload or token field. */
export interface Profile { id: string; name: string; identity: { email: string | null; plan: string | null }; createdAt: number; lastUsedAt: number | null; lastSyncedAt: number }
export interface ManagerState { profiles: Profile[]; activeProfileId: string | null; previousProfileId: string | null }
export interface Settings { vscodePath: string | null; codexPath: string | null }
export interface Diagnostics { windows: boolean; home: string; authPresent: boolean; configPresent: boolean; storage: "file" | "auto" | "keyring" | "ephemeral" | "unspecified"; vscodePath: string | null; codexPath: string | null; legacyMaintenancePresent: boolean; legacyProfilesPresent: boolean; recoveryPending: boolean }
export interface Snapshot { state: ManagerState; settings: Settings; diagnostics: Diagnostics; ipcReady: boolean }
export interface UsageWindow { kind: "primary" | "secondary"; usedPercent: number; durationMins: number | null; resetsAt: number | null }
export interface UsageSnapshot { profileId: string; checkedAt: number; windows: UsageWindow[]; ordinaryUsageAllowed: boolean | null }
export interface ProfileEvent { version: 1; type: "STATUS" | "PROFILE_CHANGED"; homeKey: string; profileId: string | null; profileName: string | null; timestamp: number }

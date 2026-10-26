$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
function Invoke-Checked([string]$Tool, [string[]]$ToolArgs) {
    & $Tool @ToolArgs
    if ($LASTEXITCODE -ne 0) { throw "$Tool failed ($LASTEXITCODE)" }
}
if (-not [System.Environment]::Is64BitOperatingSystem) { throw 'Windows x64 is required.' }
Invoke-Checked npm @('ci')
Invoke-Checked npm @('run', 'typecheck')
Invoke-Checked npm @('test')
Invoke-Checked npm @('run', 'check:secrets')
Invoke-Checked cargo @('test', '--workspace', '--exclude', 'codex-account-manager', '--locked')
Invoke-Checked npm @('run', 'desktop:build')
Invoke-Checked npm @('run', 'extension:package')

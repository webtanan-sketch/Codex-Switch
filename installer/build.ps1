param([switch]$ExecutableOnly)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
function Invoke-Checked([string]$Tool, [string[]]$ToolArgs) {
    & $Tool @ToolArgs
    if ($LASTEXITCODE -ne 0) { throw "$Tool failed ($LASTEXITCODE)" }
}
if (-not [System.Environment]::Is64BitOperatingSystem) { throw 'Windows x64 is required.' }
Invoke-Checked npm.cmd @('ci')
Invoke-Checked npm.cmd @('run', 'typecheck')
Invoke-Checked npm.cmd @('test')
Invoke-Checked npm.cmd @('run', 'check:secrets')
Invoke-Checked cargo @('test', '--workspace', '--exclude', 'codex-account-manager', '--locked')
if ($ExecutableOnly) { Invoke-Checked npm.cmd @('run', 'desktop:build', '--', '--no-bundle') }
else { Invoke-Checked npm.cmd @('run', 'desktop:build') }
Invoke-Checked npm.cmd @('run', 'extension:package')

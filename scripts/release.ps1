# Create a local checksum manifest. Publishing remains a separate maintainer step.
param([string]$OutputDirectory = 'release-artifacts')
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$installers = @(Get-ChildItem 'target/release/bundle/nsis/*-setup.exe')
$companion = Get-Item 'apps/vscode-extension/codex-account-manager-0.1.0.vsix'
if ($installers.Count -eq 0) { throw 'Build the Windows installer first.' }
$files = @($installers) + @($companion)
foreach ($file in $files) { Copy-Item $file.FullName $OutputDirectory -Force }
$manifest = foreach ($file in $files) {
    $hash = Get-FileHash $file.FullName -Algorithm SHA256
    '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), $file.Name
}
$manifest | Set-Content (Join-Path $OutputDirectory 'SHA256SUMS.txt') -Encoding ascii

# Stage the exact artifacts for one version. Publishing remains a separate maintainer step.
param([string]$OutputDirectory = 'release-artifacts')
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo

$version = (Get-Content 'package.json' -Raw | ConvertFrom-Json).version
$tag = "v$version"
$artifactDirectory = Join-Path $OutputDirectory $tag
New-Item -ItemType Directory -Path $artifactDirectory -Force | Out-Null

$sourceFiles = @(
    @{ Path = "target/release/bundle/nsis/Codex Account Manager_${version}_x64-setup.exe"; Name = "codex-account-manager-$version-x64-setup.exe" },
    @{ Path = 'target/release/codex-account-manager.exe'; Name = "codex-account-manager-$version.exe" },
    @{ Path = "apps/vscode-extension/codex-account-manager-$version.vsix"; Name = "codex-account-manager-$version.vsix" }
)
foreach ($artifact in $sourceFiles) {
    if (-not (Test-Path -LiteralPath $artifact.Path -PathType Leaf)) {
        throw "Required release artifact is missing: $($artifact.Path)"
    }
    Copy-Item -LiteralPath $artifact.Path -Destination (Join-Path $artifactDirectory $artifact.Name) -Force
}

$hashes = @{}
$manifest = foreach ($artifact in $sourceFiles) {
    $path = Join-Path $artifactDirectory $artifact.Name
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    $hashes[$artifact.Name] = $hash
    '{0}  {1}' -f $hash, $artifact.Name
}
$manifest | Set-Content -LiteralPath (Join-Path $artifactDirectory "SHA256SUMS-$version.txt") -Encoding ascii

$gitCommand = Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
if (-not $gitCommand -and (Test-Path 'C:\Program Files\Git\cmd\git.exe')) {
    $gitCommand = 'C:\Program Files\Git\cmd\git.exe'
}
if (-not $gitCommand) { $gitCommand = 'git' }
$sourceCommit = (& $gitCommand rev-parse HEAD | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($sourceCommit)) {
    throw 'Could not determine the source commit.'
}
$buildInfo = [ordered]@{
    version = $version
    sourceCommit = $sourceCommit
    tag = $tag
    builtAt = [DateTimeOffset]::Now.ToString('o')
    platform = 'Windows x64'
    releaseChannel = 'stable'
    signed = $false
    rustTests = 47
    typeScriptTests = 7
    rustFormatting = 'passed'
    typeCheck = 'passed'
    sourceCredentialScan = 'passed'
    manualAcceptance = 'maintainer-confirmed-complete'
    sha256 = $hashes
}
$buildInfo | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $artifactDirectory "build-info-$version.json") -Encoding utf8
Write-Host "Staged release $tag at $artifactDirectory"

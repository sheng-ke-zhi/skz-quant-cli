# Run after a desktop installation, without adding runtimes to this shell's PATH.
param([string]$InstallDir = "$env:LOCALAPPDATA\Programs\SKZ")
$ErrorActionPreference = 'Stop'
$cli = Join-Path $InstallDir 'skz.exe'
if (!(Test-Path $cli)) { throw 'CLI missing' }
$status = & $cli plugin status workbuddy | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or !$status.installed -or !$status.content_ok -or !$status.native_ok -or $status.needs_upgrade) {
    throw 'Native plugin or bundle validation failed'
}
$paths = @(([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') |
    Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') })
if ($paths.Count -ne 1) { throw 'Expected exactly one user PATH entry' }
foreach ($tool in @('node', 'npm', 'scoop')) {
    if (Get-Command $tool -ErrorAction SilentlyContinue) { throw "External runtime present: $tool" }
}
$config = "$env:USERPROFILE\.workbuddy"
if ($env:WORKBUDDY_CONFIG_DIR) { $config = $env:WORKBUDDY_CONFIG_DIR }
$registry = Get-Content "$config\plugins\installed_plugins.json" -Raw | ConvertFrom-Json
$entry = @($registry.plugins.'skz@skz' | Where-Object { $_.scope -eq 'user' })
if ($entry.Count -ne 1) { throw 'Expected one native user registration' }
$skills = @(Get-ChildItem $entry[0].installPath -Filter SKILL.md -Recurse -File)
$manifest = Get-Content "$InstallDir\plugins\manifest.json" -Raw | ConvertFrom-Json
if ($skills.Count -ne $manifest.skills.Count) { throw 'Installed skill count differs from the bundle' }
$auth = & $cli auth status | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or 'present' -notin $auth.PSObject.Properties.Name) { throw 'Local auth status failed' }
[pscustomobject]@{Installed=$true;ContentOK=$true;NativeOK=$true;SkillCount=$skills.Count;PathCount=$paths.Count;ExternalRuntime=$false} | ConvertTo-Json -Compress

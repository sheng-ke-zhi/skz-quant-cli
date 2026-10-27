param([Parameter(Mandatory=$true)][string]$AppDir, [Parameter(Mandatory=$true)][string]$ResolverPath)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new()
. ([scriptblock]::Create([IO.File]::ReadAllText($ResolverPath, [Text.Encoding]::UTF8))) -Library
$root = Join-Path ([IO.Path]::GetTempPath()) ('SKZ-discovery-' + [Guid]::NewGuid().ToString('N'))
$keyPath = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\SKZDiscoveryTest-' + [Guid]::NewGuid().ToString('N')
$savedBinding = $env:SKZ_WB_BINDING_FILE
$savedOverride = $env:SKZ_WORKBUDDY_APP_DIR
$passed = 0
function Assert([bool]$Condition, [string]$Message) { if (!$Condition) { throw $Message }; $script:passed++ }
function Must-Fail([scriptblock]$Action, [string]$Contains) {
    $message = $null
    try { & $Action | Out-Null } catch { $message = $_.Exception.Message }
    Assert ($message -and $message.Contains($Contains)) "Expected error containing: $Contains; actual: $message"
}
try {
    $firstDir = Join-Path $root '中文 路径\第一份'
    $secondDir = Join-Path $root '第二份'
    foreach ($dir in @($firstDir, $secondDir)) {
        $entry = Join-Path $dir 'resources\app.asar.unpacked\cli\bin\codebuddy'
        New-Item -ItemType Directory -Force -Path (Split-Path $entry) | Out-Null
        [IO.File]::WriteAllText($entry, 'fixture')
        New-Item -ItemType HardLink -Path (Join-Path $dir 'WorkBuddy.exe') -Target (Join-Path $AppDir 'WorkBuddy.exe') | Out-Null
    }
    $first = Get-WorkBuddyCandidate (Join-Path $firstDir 'WorkBuddy.exe') 'process'
    $second = Get-WorkBuddyCandidate (Join-Path $secondDir 'WorkBuddy.exe') 'registry'
    Assert ($first.compatible -and $second.compatible) 'Genuine product metadata should validate Unicode paths'
    Assert (!(Get-WorkBuddyCandidate 'C:WorkBuddy.exe' 'test')) 'Drive-relative paths must not be read'
    [IO.File]::WriteAllText((Join-Path $root 'WorkBuddy.exe'), 'not a real app')
    Assert (!(Get-WorkBuddyCandidate (Join-Path $root 'WorkBuddy.exe') 'test')) 'Name alone must not identify the product'
    Assert ((Select-WorkBuddyDesktop @($first, $first) @($second) $null).executable -eq $first.executable) 'Running path must deduplicate and take precedence'
    Must-Fail { Select-WorkBuddyDesktop @($first, $second) @() $null } '不同位置'
    Must-Fail { Select-WorkBuddyDesktop @($first) @() $second } '上次'
    Assert ((Select-WorkBuddyDesktop @() @($second, $second) $null).executable -eq $second.executable) 'Registry views must deduplicate'
    Assert ((Select-WorkBuddyDesktop @() @() $first).executable -eq $first.executable) 'Previously selected paths should work without registry entries'
    Must-Fail { Select-WorkBuddyDesktop @() @($second) $first } '不同位置'
    Must-Fail { Select-WorkBuddyDesktop @() @() $null } '无法从'
    Remove-Item -LiteralPath (Join-Path $firstDir 'resources\app.asar.unpacked\cli\bin\codebuddy')
    $incompatible = Get-WorkBuddyCandidate $first.executable 'registry'
    Must-Fail { Select-WorkBuddyDesktop @($incompatible) @($second) $null } '缺少'
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($keyPath)
    try {
        $key.SetValue('DisplayName', 'WorkBuddy 5.8.3')
        $key.SetValue('Publisher', 'Tencent Technology (Shenzhen) Company Limited')
        $key.SetValue('DisplayIcon', '"' + $second.executable + '",0')
    } finally { $key.Dispose() }
    $native = @(Get-WorkBuddyRegistry)
    Assert (@($native | Where-Object { $_.executable -eq $second.executable }).Count -gt 0) 'Native registry must parse versioned name and quoted icon when InstallLocation is absent'
    $env:SKZ_WB_BINDING_FILE = Join-Path $root 'desktop.json'
    $env:SKZ_WORKBUDDY_APP_DIR = $secondDir
    Assert ((Resolve-WorkBuddyDesktop).source -eq 'override') 'Explicit selection must use the same validator'
    $env:SKZ_WORKBUDDY_APP_DIR = 'C:relative'
    Must-Fail { Resolve-WorkBuddyDesktop } '指定'
    $env:SKZ_WORKBUDDY_APP_DIR = ''
    function Get-WorkBuddyProcesses { }
    function Get-WorkBuddyRegistry { $second }
    [IO.File]::WriteAllText($env:SKZ_WB_BINDING_FILE, '{"executable":"Z:\\gone\\WorkBuddy.exe"}')
    Assert ((Resolve-WorkBuddyDesktop).executable -eq $second.executable) 'Stale binding should recover using authoritative registry data'
    [IO.File]::WriteAllText($env:SKZ_WB_BINDING_FILE, '{invalid')
    Must-Fail { Resolve-WorkBuddyDesktop } '损坏'
    [pscustomobject]@{Passed=$passed;RealRegistryRead=$true;UnicodeAndSpacePath=$true} | ConvertTo-Json -Compress
} finally {
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($keyPath, $false)
    $env:SKZ_WB_BINDING_FILE = $savedBinding
    $env:SKZ_WORKBUDDY_APP_DIR = $savedOverride
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}

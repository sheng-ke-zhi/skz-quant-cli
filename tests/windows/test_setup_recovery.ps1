param([string]$SetupPath = "$PSScriptRoot\..\..\scripts\windows\setup.ps1")
$ErrorActionPreference = 'Stop'
$tokens = $null; $errors = $null
$source = [IO.File]::ReadAllText((Resolve-Path $SetupPath).ProviderPath, [Text.Encoding]::UTF8)
$ast = [Management.Automation.Language.Parser]::ParseInput($source, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw $errors[0] }
# 只执行原脚本的函数和主流程；所有宿主、注册表和 PATH 操作由测试替身接管。
foreach ($statement in $ast.EndBlock.Statements) {
    if ($statement -is [Management.Automation.Language.FunctionDefinitionAst]) {
        . ([scriptblock]::Create($statement.Extent.Text))
    }
}
$main = @($ast.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.TryStatementAst] })[-1]
$root = Join-Path ([IO.Path]::GetTempPath()) ('SKZ-recovery-' + [Guid]::NewGuid().ToString('N'))
$oldOverride = $env:SKZ_WORKBUDDY_APP_DIR
$oldBundle = $env:SKZ_PLUGINS_DIR
$calls = [Collections.Generic.List[string]]::new()
function Assert([bool]$Condition, [string]$Message) { if (!$Condition) { throw $Message } }
function Write-Detail([string]$Text) { $calls.Add($Text) }
function Stop-WorkBuddy { $calls.Add('stop') }
function Start-WorkBuddy { throw 'Uninstall must not start WorkBuddy' }
function Set-UserPath([bool]$Present) { $calls.Add("path:$Present") }
function Set-Registration { throw 'Must not register a desktop program during external-plugin recovery' }
# Restore-Previous 的这两个 Remove-Item 仅针对真实用户登记与快捷方式，测试不得触及。
function Remove-Item {
    param([string[]]$LiteralPath, [switch]$Recurse, [switch]$Force, [string]$ErrorAction)
    foreach ($path in $LiteralPath) {
        if ($path.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
            Microsoft.PowerShell.Management\Remove-Item -LiteralPath $path -Recurse:$Recurse -Force:$Force -ErrorAction Stop
        }
    }
}
function Invoke-Skz([string[]]$Arguments, [string]$Executable) {
    $command = $Arguments -join ' '
    $calls.Add($command)
    if ($command -eq '__workbuddy-desktop') {
        if ($scenario -eq 'missing-host') { throw 'host removed' }
        return @{executable=(Join-Path $root 'WorkBuddy.exe');source='fixture'}
    }
    if ($command -eq '__workbuddy-setup restore' -and $failRestore) { throw 'restore failed' }
    if ($command -eq 'plugin uninstall workbuddy' -and $scenario -eq 'native-failure') { throw 'native uninstall failed' }
    return @{}
}
function Test-WorkBuddyRuntime { return $scenario -eq 'native-failure' }
try {
    $null = New-Item -ItemType Directory -Path $root
    $InstallDir = Join-Path $root 'program'
    $pluginRoot = Join-Path $root 'plugin'
    $backup = Join-Path $root 'previous'
    $journal = Join-Path $root 'transaction.json'
    $cli = Join-Path $root 'payload\skz.exe'
    $null = New-Item -ItemType Directory -Path "$backup\plugin\source\plugins\skz" -Force
    $skill = Join-Path $backup 'plugin\source\plugins\skz\old.txt'
    [IO.File]::WriteAllText($skill, 'old skills')
    $saved = [pscustomobject]@{
        format=1;install_dir=$InstallDir;state_root=$pluginRoot;program_touched=$true;plugin_touched=$true;
        had_program=$false;had_state=$true;registered=$true;market_registered=$true;enabled=$false;had_path=$false;
        backup_files=@(@{path='plugin\source\plugins\skz\old.txt';sha256=(Get-FileHash $skill).Hash})
    }
    [IO.File]::WriteAllText($journal, '{}')
    Restore-Previous $saved
    Assert ($calls.Contains('__workbuddy-setup restore')) 'External plugin must be restored without a previous desktop program'
    Assert ($calls.Contains('__workbuddy-setup disable')) 'Disabled state must be restored'
    Assert ((Get-Content "$pluginRoot\source\plugins\skz\old.txt") -eq 'old skills') 'Original plugin source must survive'
    Assert (!(Test-Path $journal)) 'Successful recovery must clear journal'
    $failRestore = $true
    [IO.File]::WriteAllText($journal, '{}')
    $failed = $false
    try { Restore-Previous $saved } catch { $failed = $true }
    Assert ($failed -and (Test-Path $journal)) 'Failed native restoration must retain recovery journal'
    Remove-Item -LiteralPath $journal -Force

    foreach ($scenario in @('missing-host', 'missing-runtime', 'native-failure')) {
        $calls.Clear()
        $Mode = 'Uninstall'; $app = $null; $lock = $null; $transaction = $null; $closed = $false
        $outcome = 'unchanged'; $stage = 'test'; $originalAppOverride = $oldOverride
        $stateRoot = Join-Path $root $scenario
        $log = Join-Path $stateRoot 'last.log'; $ResultFile = Join-Path $stateRoot 'result.txt'
        # 替换进程退出语句，保留生产脚本中的分支、异常处理和 finally。
        $body = $main.Extent.Text.Replace('exit 0', 'return').Replace('exit 1', "throw 'setup failed'")
        $failed = $false
        try { . ([scriptblock]::Create($body)) } catch { $failed = $true }
        if ($scenario -eq 'native-failure') {
            Assert $failed 'Native uninstall errors must not be swallowed'
            Assert (!$calls.Contains('path:False')) 'Failed native uninstall must not report local cleanup success'
        } else {
            Assert (!$failed -and $outcome -eq 'uninstalled') 'Missing host/runtime must allow local uninstall'
            Assert ($calls.Contains('path:False')) 'User PATH must be removed'
            Assert (!$calls.Contains('plugin uninstall workbuddy')) 'Unavailable host must not be invoked'
            Assert (@($calls | Where-Object { $_ -like '*跳过宿主插件清理*' }).Count -eq 1) 'Skipped native cleanup must be reported'
        }
    }
    'PASS: external-plugin rollback, failed recovery, missing host/runtime uninstall, native uninstall failure'
} finally {
    $env:SKZ_WORKBUDDY_APP_DIR = $oldOverride
    $env:SKZ_PLUGINS_DIR = $oldBundle
    Microsoft.PowerShell.Management\Remove-Item -LiteralPath $root -Recurse -Force
}

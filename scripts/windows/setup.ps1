param(
    [Parameter(Mandatory=$true)][string]$InstallDir,
    [string]$PayloadDir,
    [string]$ResultFile,
    [ValidateSet('Install','Uninstall')][string]$Mode = 'Install'
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
# nsExec 与 CLI 使用独立编码的管道，避免依赖系统代码页。
[Console]::OutputEncoding = [Text.Encoding]::Unicode
$InstallDir = [IO.Path]::GetFullPath($InstallDir).TrimEnd('\')
$stateRoot = "$env:LOCALAPPDATA\SKZ\Installer"
$backup = Join-Path $stateRoot 'previous'
$journal = Join-Path $stateRoot 'transaction.json'
$log = Join-Path $stateRoot 'last.log'
if (!$ResultFile) { $ResultFile = Join-Path $stateRoot 'result.txt' }
$stage = '检查安装环境'
$transaction = $null
$lock = $null
$app = $null
$closed = $false
$outcome = 'unchanged'
$originalAppOverride = $env:SKZ_WORKBUDDY_APP_DIR

function Write-Detail([string]$Text) {
    $line = "$(Get-Date -Format 'HH:mm:ss')  $Text"
    [Console]::WriteLine($line)
    try { [IO.File]::AppendAllText($log, "$line`r`n", [Text.UTF8Encoding]::new($true)) }
    catch { [Console]::WriteLine('安装详情暂时无法写入日志文件。') }
}
function Write-Json([string]$Path, $Value) {
    $temp = "$Path.tmp"
    [IO.File]::WriteAllText($temp, ($Value | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    # PowerShell 5.1 会把普通 $null 转为空路径，必须传入真正的 null string。
    if (Test-Path -LiteralPath $Path) { [IO.File]::Replace($temp, $Path, [NullString]::Value) }
    else { [IO.File]::Move($temp, $Path) }
}
function Read-Json([string]$Path) { return ([IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8) | ConvertFrom-Json) }
function Start-WorkBuddy {
    $pinned = $env:SKZ_WORKBUDDY_APP_DIR
    try {
        $env:SKZ_WORKBUDDY_APP_DIR = $originalAppOverride
        Start-Process -FilePath $app -WorkingDirectory (Split-Path -Parent $app)
    } finally { $env:SKZ_WORKBUDDY_APP_DIR = $pinned }
}
function Start-HiddenProcess([string]$Executable, [string]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    $info.Arguments = $Arguments
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.StandardOutputEncoding = [Text.Encoding]::UTF8
    $info.StandardErrorEncoding = [Text.Encoding]::UTF8
    return [Diagnostics.Process]::Start($info)
}
function Stop-Tree([int]$ProcessId) {
    $killer = Start-HiddenProcess "$env:SystemRoot\System32\taskkill.exe" "/PID $ProcessId /T /F"
    try {
        $output = $killer.StandardOutput.ReadToEndAsync()
        $errors = $killer.StandardError.ReadToEndAsync()
        if (!$killer.WaitForExit(10000)) { $killer.Kill(); throw '关闭安装子进程超时。' }
        $null = $output.GetAwaiter().GetResult()
        $null = $errors.GetAwaiter().GetResult()
    } finally { $killer.Dispose() }
}
function Invoke-Skz([string[]]$Arguments, [string]$Executable = $cli) {
    $process = Start-HiddenProcess $Executable ($Arguments -join ' ')
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (!$process.WaitForExit(120000)) {
            Stop-Tree $process.Id
            throw '胜可知安装命令执行超时。'
        }
        $text = $stdout.GetAwaiter().GetResult()
        $errorText = $stderr.GetAwaiter().GetResult()
        if ($process.ExitCode -ne 0) {
            try { $message = ($errorText | ConvertFrom-Json).error.message } catch { $message = '胜可知安装命令未完成。' }
            throw $message
        }
        return ($text | ConvertFrom-Json)
    } finally { $process.Dispose() }
}
function Assert-Idle {
    $deadline = (Get-Date).AddSeconds(10)
    do {
        $busy = @(Get-CimInstance Win32_Process -Filter "Name='skz.exe'" | Where-Object {
            $_.ExecutablePath -ieq (Join-Path $InstallDir 'skz.exe')
        })
        if (!$busy.Count) { return }
        Start-Sleep -Milliseconds 300
    } while ((Get-Date) -lt $deadline)
    throw '胜可知还有任务正在执行，请等待任务结束后再升级。'
}
function Stop-WorkBuddy {
    Assert-Idle
    $session = (Get-Process -Id $PID).SessionId
    $apps = @(Get-CimInstance Win32_Process -Filter "Name='WorkBuddy.exe'" | Where-Object {
        $_.SessionId -eq $session -and $_.ExecutablePath -ieq $app
    })
    if ($apps.Count) { $script:closed = $true }
    foreach ($item in $apps) {
        $process = Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
        if ($process -and $process.MainWindowHandle -ne 0) { $null = $process.CloseMainWindow() }
    }
    $deadline = (Get-Date).AddSeconds(15)
    do {
        $windows = @($apps | ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue } |
            Where-Object { $_.MainWindowHandle -ne 0 })
        if (!$windows.Count) { break }
        Start-Sleep -Milliseconds 300
    } while ((Get-Date) -lt $deadline)
    if ($windows.Count) { throw 'WorkBuddy 尚未退出，请保存工作并关闭它后重试。' }
    # 主窗口正常关闭后，释放仍存活的后台进程及插件缓存租约。
    foreach ($item in $apps) {
        $alive = Get-CimInstance Win32_Process -Filter "ProcessId=$($item.ProcessId)" -ErrorAction SilentlyContinue
        if ($alive -and $alive.CreationDate -eq $item.CreationDate) {
            Stop-Tree $item.ProcessId
        }
    }
    $script:closed = $true
    Assert-Idle
}
function Test-WorkBuddyRuntime {
    $roots = @("$env:USERPROFILE\.workbuddy")
    if ($env:WORKBUDDY_CONFIG_DIR) { $roots = @($env:WORKBUDDY_CONFIG_DIR) + $roots }
    $nodes = @($roots | ForEach-Object {
        Get-ChildItem "$_\binaries\node\versions\*\node.exe" -File -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
    })
    $systemNode = Get-Command node -ErrorAction SilentlyContinue
    if ($systemNode) { $nodes += $systemNode.Source }
    foreach ($node in $nodes) {
        $process = $null
        try {
            $process = Start-HiddenProcess $node '--version'
            $output = $process.StandardOutput.ReadToEndAsync()
            $errors = $process.StandardError.ReadToEndAsync()
            if ($process.WaitForExit(5000)) {
                $null = $output.GetAwaiter().GetResult(); $null = $errors.GetAwaiter().GetResult()
                if ($process.ExitCode -eq 0) { return $true }
            } else { $process.Kill() }
        } catch { } finally { if ($process) { $process.Dispose() } }
    }
    return $false
}
function Set-UserPath([bool]$Present) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ -and $_.TrimEnd('\') -ine $InstallDir })
    if ($Present) { $entries = @($InstallDir) + $entries }
    [Environment]::SetEnvironmentVariable('Path', ($entries -join ';'), 'User')
    $env:Path = "$InstallDir;$env:Path"
    if (!('SkzEnvironmentChange' -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public class SkzEnvironmentChange {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern IntPtr SendMessageTimeout(IntPtr h, uint m, UIntPtr w, string l, uint f, uint t, out UIntPtr r);
}
'@
    }
    $result = [UIntPtr]::Zero
    $null = [SkzEnvironmentChange]::SendMessageTimeout([IntPtr]0xffff,0x1a,[UIntPtr]::Zero,'Environment',2,3000,[ref]$result)
}
function Set-Registration([string]$Version) {
    $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\SKZWorkBuddy'
    $null = New-Item $key -Force
    foreach ($entry in @{
        DisplayName='胜可知 WorkBuddy 程序与技能'; DisplayVersion=$Version; Publisher='胜可知';
        InstallLocation=$InstallDir; UninstallString=('"' + (Join-Path $InstallDir '卸载 SKZ.exe') + '"')
    }.GetEnumerator()) { Set-ItemProperty $key $entry.Key $entry.Value }
    Set-ItemProperty $key NoModify 1 -Type DWord
    Set-ItemProperty $key NoRepair 1 -Type DWord
    if (Test-Path -LiteralPath (Join-Path $InstallDir 'update.ps1')) {
        $shell = New-Object -ComObject WScript.Shell
        Remove-Item -LiteralPath "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级 SKZ.lnk" -Force -ErrorAction SilentlyContinue
        # CLI 依赖安装目录中的固定入口；开始菜单使用中文名称。
        foreach ($path in @((Join-Path $InstallDir '升级 SKZ.lnk'), "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级胜可知.lnk")) {
            $shortcut = $shell.CreateShortcut($path)
            $shortcut.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
            $shortcut.Arguments = '-NoLogo -NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + (Join-Path $InstallDir 'update.ps1') + '"'
            $shortcut.WorkingDirectory = $InstallDir
            $shortcut.Description = '为 WorkBuddy 检查并升级胜可知程序和技能'
            $shortcut.Save()
        }
    } else {
        Remove-Item -LiteralPath "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级 SKZ.lnk","$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级胜可知.lnk" -Force -ErrorAction SilentlyContinue
    }
}
function Copy-Directory([string]$From, [string]$To) {
    $null = New-Item -ItemType Directory -Path $To -Force
    Get-ChildItem -LiteralPath $From -Force | Copy-Item -Destination $To -Recurse -Force
}
function Clear-Directory([string]$Directory) {
    # Windows 终端或 Shell 可占用目录本身；保留目录，只替换已备份的自有内容。
    if (Test-Path -LiteralPath $Directory) {
        Get-ChildItem -LiteralPath $Directory -Force | ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force }
    }
}
function Assert-Plugin([string]$Executable) {
    $status = Invoke-Skz @('plugin','status','workbuddy') $Executable
    if (!$status.installed -or !$status.content_ok -or !$status.native_ok -or $status.needs_upgrade) {
        throw '程序、技能内容或 WorkBuddy 实际登记检查未通过。'
    }
    return $status
}
function Restore-Previous($Saved) {
    if ($Saved.format -ne 1 -or $Saved.install_dir -ine $InstallDir -or $Saved.state_root -ine $pluginRoot -or
        $Saved.program_touched -isnot [bool] -or $Saved.plugin_touched -isnot [bool] -or $Saved.had_program -isnot [bool] -or
        $Saved.had_state -isnot [bool] -or $Saved.registered -isnot [bool] -or $Saved.had_path -isnot [bool]) {
        throw '升级恢复记录无效或与当前安装位置不一致，已保留备份。'
    }
    if (!$Saved.program_touched) { Remove-Item -LiteralPath $journal -Force; return }
    Write-Detail '正在校验上一套程序和技能的备份。'
    if ($Saved.had_program -and (!(Test-Path -LiteralPath (Join-Path $backup 'program\.skz-desktop-install')) -or !@($Saved.backup_files).Count)) {
        throw '旧版备份不完整，未执行恢复覆盖。'
    }
    foreach ($file in $Saved.backup_files) {
        $path = [IO.Path]::GetFullPath((Join-Path $backup $file.path))
        if (!$path.StartsWith($backup + '\', [StringComparison]::OrdinalIgnoreCase) -or
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) { throw '旧版备份校验失败，未执行恢复覆盖。' }
    }
    Stop-WorkBuddy
    if ($Saved.plugin_touched) {
        $null = Invoke-Skz @('__workbuddy-setup','reset')
        if ($Saved.market_registered -eq $false) { $null = Invoke-Skz @('plugin','uninstall','workbuddy') }
    }
    Clear-Directory $pluginRoot
    if ($Saved.had_state) { Copy-Directory (Join-Path $backup 'plugin') $pluginRoot }
    Clear-Directory $InstallDir
    if ($Saved.had_program) {
        Copy-Directory (Join-Path $backup 'program') $InstallDir
        $oldCli = Join-Path $InstallDir 'skz.exe'
        if ((Get-FileHash -LiteralPath $oldCli -Algorithm SHA256).Hash -ine $Saved.old_sha256) { throw '旧版程序恢复校验失败，已保留备份。' }
        Write-Detail '旧版程序已恢复，正在恢复 WorkBuddy 中的技能。'
        if ($Saved.plugin_touched -and $Saved.registered) {
            # 同版修复可能已更新同名缓存；用新执行器恢复旧 bundle，兼容旧执行器没有强制修复接口。
            $restoreCli = $oldCli
            $previousBundle = $env:SKZ_PLUGINS_DIR
            try {
                if ($Saved.old_version -eq $version.cli) {
                    $restoreCli = $cli
                    $env:SKZ_PLUGINS_DIR = Join-Path $InstallDir 'plugins'
                }
                $null = Invoke-Skz @('plugin','install','workbuddy') $restoreCli
            } finally { $env:SKZ_PLUGINS_DIR = $previousBundle }
            if ($Saved.enabled -eq $false) { $null = Invoke-Skz @('__workbuddy-setup','disable') }
        }
        if ($Saved.healthy) { $null = Assert-Plugin $oldCli }
        Set-Registration $Saved.old_version
    } else {
        if ($Saved.plugin_touched -and $Saved.registered) {
            $null = Invoke-Skz @('__workbuddy-setup','restore')
            if ($Saved.enabled -eq $false) { $null = Invoke-Skz @('__workbuddy-setup','disable') }
        }
        Remove-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\SKZWorkBuddy' -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级 SKZ.lnk","$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级胜可知.lnk" -Force -ErrorAction SilentlyContinue
    }
    Set-UserPath $Saved.had_path
    Remove-Item -LiteralPath $journal -Force
    Write-Detail '已恢复升级前的程序和技能；账户数据未参与恢复。'
}

try {
    $null = New-Item -ItemType Directory -Path $stateRoot -Force
    $lock = [IO.File]::Open((Join-Path $stateRoot 'install.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    if (Test-Path -LiteralPath $log) { Copy-Item -LiteralPath $log -Destination (Join-Path $stateRoot ('install-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff') + '.log')) }
    [IO.File]::WriteAllText($log, '', [Text.UTF8Encoding]::new($true))
    Write-Detail '开始检查；账户凭证、聊天记录和其他插件不在安装范围内。'
    $env:SKZ_PLUGINS_DIR = $null
    if ($Mode -eq 'Install') {
        $PayloadDir = [IO.Path]::GetFullPath($PayloadDir).TrimEnd('\')
        $cli = Join-Path $PayloadDir 'skz.exe'
        $stage = '检查安装包'
        $package = Read-Json (Join-Path $PayloadDir 'installer-manifest.json')
        foreach ($file in $package.files) {
            if ($file.path -notmatch '^[a-zA-Z0-9_. /-]+$' -or $file.path -match '(^|/)\.\.(/|$)' -or $file.path.StartsWith('/')) { throw '安装包含无效路径。' }
            $path = Join-Path $PayloadDir $file.path
            if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) { throw '安装包文件校验失败，原版本未修改。' }
        }
        $version = Invoke-Skz @('--version')
        if ($package.cli -ne $version.cli -or $version.cli -notmatch '^\d+\.\d+\.\d+$') { throw '安装包与程序版本不一致。' }
        Write-Detail "安装包检查通过：胜可知 $($version.cli)。"
    } else { $cli = Join-Path $InstallDir 'skz.exe' }

    if ((Test-Path -LiteralPath $InstallDir) -and !(Test-Path -LiteralPath $journal)) {
        $items = @(Get-ChildItem -LiteralPath $InstallDir -Force)
        if ($items.Count -and (!(Test-Path -LiteralPath (Join-Path $InstallDir '.skz-desktop-install')) -or
            [IO.File]::ReadAllText((Join-Path $InstallDir '.skz-desktop-install')).Trim() -ne 'SKZ WorkBuddy desktop installer')) {
            throw '安装目录已有其他来源的文件，已停止，未覆盖。'
        }
    }
    $stage = '定位 WorkBuddy'
    try {
        $arguments = @('__workbuddy-desktop')
        if ($Mode -eq 'Install') { $arguments += '--interactive' }
        $desktop = Invoke-Skz $arguments
        $app = $desktop.executable
        $env:SKZ_WORKBUDDY_APP_DIR = Split-Path -Parent $app
        Write-Detail "已找到 WorkBuddy（$($desktop.source)）：$app"
        if ($Mode -eq 'Uninstall' -and !(Test-WorkBuddyRuntime)) { throw 'WorkBuddy 运行环境不可用。' }
    } catch {
        if ($Mode -ne 'Uninstall') { throw }
        $app = $null
        Write-Detail "无法使用 WorkBuddy：$($_.Exception.Message)"
    }
    $stage = '准备 WorkBuddy 运行环境'
    if ($Mode -eq 'Install' -and !(Test-WorkBuddyRuntime)) {
        Write-Detail '正在准备 WorkBuddy 首次运行环境……'
        Start-WorkBuddy
        $deadline = (Get-Date).AddSeconds(90)
        while (!(Test-WorkBuddyRuntime)) {
            if ((Get-Date) -ge $deadline) { throw 'WorkBuddy 首次环境准备超时，请稍后重新运行安装包。' }
            Start-Sleep -Seconds 2
        }
    }
    if ($Mode -eq 'Install') {
        $preflight = Invoke-Skz @('__workbuddy-setup','check')
        $pluginRoot = $preflight.state_root
    }

    if (Test-Path -LiteralPath $journal) {
        if ($Mode -eq 'Uninstall') { throw '有未完成的升级，请先重新运行安装包恢复，再卸载。' }
        Write-Detail '发现未完成的升级，先恢复上一套程序和技能。'
        $saved = Read-Json $journal
        Restore-Previous $saved
        $preflight = Invoke-Skz @('__workbuddy-setup','check')
    }
    if ($Mode -eq 'Uninstall') {
        $stage = '卸载胜可知技能'
        if ($app) {
            Stop-WorkBuddy
            $null = Invoke-Skz @('plugin','uninstall','workbuddy')
        } else {
            Write-Detail '跳过宿主插件清理：保留 WorkBuddy 登记、缓存和 SKZ 插件状态；继续移除本机程序和用户路径。'
        }
        Set-UserPath $false
        Remove-Item -LiteralPath "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级 SKZ.lnk","$env:APPDATA\Microsoft\Windows\Start Menu\Programs\升级胜可知.lnk" -Force -ErrorAction SilentlyContinue
        Write-Detail '用户路径已移除；账户登录信息保留。'
        $outcome = 'uninstalled'
    } else {
        $oldCli = Join-Path $InstallDir 'skz.exe'
        $old = $null
        $healthy = $false
        if (Test-Path -LiteralPath $oldCli) {
            $old = Invoke-Skz @('--version') $oldCli
            if ([Version]$old.cli -gt [Version]$version.cli) { throw '本机版本较新，已停止降级，原版本未修改。' }
            try { $null = Assert-Plugin $oldCli; $healthy = $true } catch { }
        }
        if ($old -and $old.cli -eq $version.cli -and $healthy -and
            (Get-FileHash -LiteralPath $oldCli -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash -and
            (Test-Path -LiteralPath (Join-Path $InstallDir 'installer-manifest.json')) -and
            (Get-FileHash -LiteralPath (Join-Path $InstallDir 'installer-manifest.json') -Algorithm SHA256).Hash -eq
                (Get-FileHash -LiteralPath (Join-Path $PayloadDir 'installer-manifest.json') -Algorithm SHA256).Hash) {
            Write-Detail "当前胜可知 $($version.cli) 程序与技能完整，无需升级。"
            Set-Registration $version.cli
            Set-UserPath $true
        } else {
            $stage = '保存上一版'
            Assert-Idle
            if (Test-Path -LiteralPath $backup) { Remove-Item -LiteralPath $backup -Recurse -Force }
            $null = New-Item -ItemType Directory -Path $backup
            if ($old) { Copy-Directory $InstallDir (Join-Path $backup 'program') }
            $hadState = Test-Path -LiteralPath $pluginRoot
            if ($hadState) { Copy-Directory $pluginRoot (Join-Path $backup 'plugin') }
            $backupFiles = @(Get-ChildItem -LiteralPath $backup -Recurse -File -Force | ForEach-Object {
                @{path=$_.FullName.Substring($backup.Length + 1);sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}
            })
            $transaction = [pscustomobject]@{
                format=1;install_dir=$InstallDir; state_root=$pluginRoot; had_program=[bool]$old; had_state=$hadState;
                old_version=$(if ($old) { $old.cli } else { $null }); old_sha256=$(if ($old) { (Get-FileHash -LiteralPath $oldCli -Algorithm SHA256).Hash } else { $null });
                registered=$preflight.registered; market_registered=$preflight.market_registered; enabled=$preflight.enabled; healthy=$healthy; plugin_touched=$false; program_touched=$false; backup_files=$backupFiles;
                had_path=($InstallDir -in @([Environment]::GetEnvironmentVariable('Path','User') -split ';' | ForEach-Object { $_.TrimEnd('\') }))
            }
            Write-Json $journal $transaction
            Write-Detail '上一版已保存，开始关闭 WorkBuddy。'
            $stage = '关闭 WorkBuddy'
            Stop-WorkBuddy
            $stage = '更新胜可知程序'
            $transaction.program_touched = $true
            Write-Json $journal $transaction
            Clear-Directory $InstallDir
            Copy-Directory $PayloadDir $InstallDir
            [IO.File]::WriteAllText((Join-Path $InstallDir '.skz-desktop-install'), 'SKZ WorkBuddy desktop installer')
            Write-Detail "程序已更新：胜可知 $($version.cli)。"
            $stage = '更新并检查技能'
            $transaction.plugin_touched = $true
            Write-Json $journal $transaction
            $installedCli = Join-Path $InstallDir 'skz.exe'
            $null = Invoke-Skz @('plugin','install','workbuddy') $installedCli
            $null = Assert-Plugin $installedCli
            $actual = Invoke-Skz @('--version') $installedCli
            if ($actual.cli -ne $version.cli) { throw '安装后的程序版本检查失败。' }
            $after = Invoke-Skz @('__workbuddy-setup','check') $installedCli
            if ($preflight.enabled -eq $false -and $after.enabled -ne $false) { throw '插件原有的禁用状态没有保留。' }
            Write-Detail "程序与技能检查通过：胜可知 $($actual.cli)，$(@($after.skills).Count) 个技能。"
            $stage = '完成升级登记'
            Set-UserPath $true
            Set-Registration $version.cli
            $outcome = 'installed'
        }
        $stage = '重新打开 WorkBuddy'
        Start-WorkBuddy
        Write-Detail 'WorkBuddy 已请求重新打开，请在新对话中继续使用胜可知。'
    }
    Write-Json (Join-Path $stateRoot 'result.json') @{outcome=$outcome;stage=$stage;log=$log}
    if ($transaction) { Remove-Item -LiteralPath $journal -Force; $transaction = $null }
    exit 0
} catch {
    $reason = $_.Exception.Message
    $failedStage = $stage
    $message = "$failedStage 未完成。`r`n$reason"
    if ($lock) {
        Write-Detail $message
        if ($transaction) {
            try {
                Restore-Previous $transaction
                if ($transaction.program_touched) {
                    $outcome = 'restored'
                    $message += "`r`n`r`n已恢复升级前的程序和技能。"
                } else { $message += "`r`n`r`n原有程序和技能未被替换。" }
            } catch {
                $outcome = 'recovery_required'
                $message += "`r`n`r`n恢复未完成：$($_.Exception.Message)`r`n备份已保留，请重新运行本安装包恢复。"
            }
        } elseif (Test-Path -LiteralPath $journal) {
            $outcome = 'recovery_required'
            $message += "`r`n`r`n上次升级尚未恢复；备份已保留，请重新运行本安装包。"
        } else { $message += "`r`n`r`n原有程序和账户数据未被替换。" }
        if ($closed -and $app -and $outcome -ne 'recovery_required') {
            try { Start-WorkBuddy } catch { }
        }
        Write-Detail "结果：$outcome"
        try { Write-Json (Join-Path $stateRoot 'result.json') @{outcome=$outcome;stage=$failedStage;log=$log} } catch { }
    }
    $message += "`r`n`r`n查看详情：$log"
    [IO.File]::WriteAllText($ResultFile, $message, [Text.UnicodeEncoding]::new($false,$false))
    exit 1
} finally { if ($lock) { $lock.Dispose() } }

param([switch]$Library, [switch]$Interactive)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new()

function Get-WorkBuddyCandidate([string]$Executable, [string]$Source) {
    if (!$Executable -or $Executable -notmatch '^(?:[A-Za-z]:\\|\\\\)') { return }
    try {
        $file = Get-Item -LiteralPath $Executable -ErrorAction Stop
        if ($file.PSIsContainer -or $file.Name -ine 'WorkBuddy.exe') { return }
        if ($file.VersionInfo.ProductName -ne 'WorkBuddy' -or
            $file.VersionInfo.CompanyName -ne 'Tencent Technology (Shenzhen) Company Limited') { return }
        [pscustomobject]@{
            executable = $file.FullName
            source = $Source
            compatible = Test-Path -LiteralPath (Join-Path $file.DirectoryName 'resources\app.asar.unpacked\cli\bin\codebuddy') -PathType Leaf
        }
    } catch { return }
}

function Get-WorkBuddyProcesses {
    $session = (Get-Process -Id $PID).SessionId
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    try {
        foreach ($process in Get-CimInstance Win32_Process -Filter "Name='WorkBuddy.exe' AND SessionId=$session") {
            try {
                if ((Invoke-CimMethod -InputObject $process -MethodName GetOwnerSid).Sid -eq $sid) {
                    Get-WorkBuddyCandidate $process.ExecutablePath 'process'
                }
            } catch { }
        }
    } catch { }
}

function Get-WorkBuddyRegistry {
    foreach ($hive in @([Microsoft.Win32.RegistryHive]::CurrentUser, [Microsoft.Win32.RegistryHive]::LocalMachine)) {
        foreach ($view in @([Microsoft.Win32.RegistryView]::Registry64, [Microsoft.Win32.RegistryView]::Registry32)) {
            $base = $null; $apps = $null; $uninstall = $null
            try {
                $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, $view)
                $apps = $base.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\App Paths\WorkBuddy.exe')
                if ($apps) {
                    $appPath = $apps.GetValue('') -as [string]
                    if ($appPath) { Get-WorkBuddyCandidate $appPath.Trim('"') 'registry' }
                }
                $uninstall = $base.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Uninstall')
                if (!$uninstall) { continue }
                foreach ($name in $uninstall.GetSubKeyNames()) {
                    $key = $null
                    try {
                        $key = $uninstall.OpenSubKey($name)
                        if ($key.GetValue('DisplayName') -notmatch '^WorkBuddy(?:\s+\d[^\r\n]*)?$' -or
                            $key.GetValue('Publisher') -ne 'Tencent Technology (Shenzhen) Company Limited') { continue }
                        $location = $key.GetValue('InstallLocation') -as [string]
                        if ($location) { Get-WorkBuddyCandidate (Join-Path $location.Trim('"') 'WorkBuddy.exe') 'registry' }
                        $icon = $key.GetValue('DisplayIcon') -as [string]
                        if ($icon) {
                            # 图标字段可以附带资源编号，只接受明确的 EXE 路径，不执行登记中的命令。
                            $executable = ($icon -replace ',\s*-?\d+\s*$', '').Trim().Trim('"')
                            Get-WorkBuddyCandidate $executable 'registry'
                        }
                    } catch { } finally { if ($key) { $key.Dispose() } }
                }
            } catch { } finally {
                foreach ($key in @($apps, $uninstall, $base)) { if ($key) { $key.Dispose() } }
            }
        }
    }
}

function Select-WorkBuddyDesktop($Running, $Registered, $Saved) {
    $runningPaths = @($Running | Sort-Object executable -Unique)
    if ($runningPaths.Count -gt 1) { throw '当前会话运行着不同位置的 WorkBuddy，请关闭不用的版本，或选择本次要使用的程序。' }
    if ($runningPaths.Count -eq 1) {
        if (!$runningPaths[0].compatible) { throw '正在运行的 WorkBuddy 缺少技能安装接口，请更新 WorkBuddy，或选择其他受支持版本。' }
        if ($Saved -and $Saved.executable -ine $runningPaths[0].executable) {
            throw '正在运行的 WorkBuddy 与上次安装 SKZ 时的位置不同，请选择本次要使用的程序。'
        }
        return $runningPaths[0]
    }
    $candidates = @(@($Registered) + @($Saved) | Where-Object { $_ } | Sort-Object executable -Unique)
    if ($candidates.Count -gt 1) { throw '发现不同位置的 WorkBuddy 安装记录，请打开您要使用的版本后重新检测，或选择程序。' }
    if ($candidates.Count -eq 1) {
        if (!$candidates[0].compatible) { throw '已找到 WorkBuddy，但该版本缺少技能安装接口，请更新 WorkBuddy。' }
        return $candidates[0]
    }
    throw '无法从当前会话或 Windows 安装登记确认 WorkBuddy 位置。请打开您平时使用的 WorkBuddy 后重新检测，或选择 WorkBuddy.exe。'
}

function Resolve-WorkBuddyDesktop {
    if ($env:SKZ_WORKBUDDY_APP_DIR) {
        $explicit = Get-WorkBuddyCandidate (Join-Path $env:SKZ_WORKBUDDY_APP_DIR 'WorkBuddy.exe') 'override'
        if (!$explicit -or !$explicit.compatible) { throw '指定的 WorkBuddy 位置无效，或缺少技能安装接口，请重新选择程序。' }
        return $explicit
    }
    $saved = $null
    if (Test-Path -LiteralPath $env:SKZ_WB_BINDING_FILE) {
        try {
            $binding = Get-Content -LiteralPath $env:SKZ_WB_BINDING_FILE -Raw -Encoding UTF8 | ConvertFrom-Json
            $saved = Get-WorkBuddyCandidate $binding.executable 'saved'
        } catch { throw '保存的 WorkBuddy 位置信息损坏，请重新选择程序。' }
    }
    Select-WorkBuddyDesktop @(Get-WorkBuddyProcesses) @(Get-WorkBuddyRegistry) $saved
}

function Show-WorkBuddyChoice([string]$Reason) {
    Add-Type -AssemblyName System.Windows.Forms
    $form = New-Object Windows.Forms.Form
    $form.Text = '选择 WorkBuddy'; $form.ClientSize = New-Object Drawing.Size(560, 200)
    $form.StartPosition = 'CenterScreen'; $form.TopMost = $true
    $form.FormBorderStyle = 'FixedDialog'; $form.MaximizeBox = $false; $form.MinimizeBox = $false
    $form.Font = New-Object Drawing.Font('Microsoft YaHei UI', 10)
    $label = New-Object Windows.Forms.Label
    $label.Text = $Reason; $label.Location = New-Object Drawing.Point(20, 20)
    $label.Size = New-Object Drawing.Size(520, 110); $form.Controls.Add($label)
    foreach ($spec in @(@('重新检测', 'Retry', 20), @('选择程序…', 'Yes', 190), @('取消安装', 'Cancel', 360))) {
        $button = New-Object Windows.Forms.Button
        $button.Text = $spec[0]; $button.DialogResult = $spec[1]
        $button.Location = New-Object Drawing.Point($spec[2], 150)
        $button.Size = New-Object Drawing.Size(150, 34); $form.Controls.Add($button)
    }
    try { $choice = $form.ShowDialog() } finally { $form.Dispose() }
    if ($choice -eq 'Retry') { return $null }
    if ($choice -ne 'Yes') { throw '安装已取消，尚未安装或更新 WorkBuddy 技能。' }
    $dialog = New-Object Windows.Forms.OpenFileDialog
    $dialog.Title = '请选择您使用的 WorkBuddy.exe'
    $dialog.Filter = 'WorkBuddy 程序 (WorkBuddy.exe)|WorkBuddy.exe'
    try {
        if ($dialog.ShowDialog() -ne 'OK') { throw '安装已取消，尚未安装或更新 WorkBuddy 技能。' }
        $candidate = Get-WorkBuddyCandidate $dialog.FileName 'selected'
        if (!$candidate -or !$candidate.compatible) { throw '选择的文件不是受支持的 WorkBuddy 桌面程序，请重新运行安装包选择。' }
        return $candidate
    } finally { $dialog.Dispose() }
}

if (!$Library) {
    try {
        while ($true) {
            try { $desktop = Resolve-WorkBuddyDesktop; break } catch {
                if (!$Interactive) { throw }
                $desktop = Show-WorkBuddyChoice $_.Exception.Message
                if ($desktop) { break }
            }
        }
        $desktop | Select-Object executable, source | ConvertTo-Json -Compress
    } catch {
        [Console]::Error.WriteLine($_.Exception.Message)
        exit 2
    }
}

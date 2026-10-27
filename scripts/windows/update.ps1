$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[Windows.Forms.Application]::EnableVisualStyles()
$form = [Windows.Forms.Form]::new()
$form.Text = '升级胜可知 · WorkBuddy'
$form.Size = [Drawing.Size]::new(570,250)
$form.StartPosition = 'CenterScreen'
$form.Font = [Drawing.Font]::new('Microsoft YaHei UI',11)
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$label = [Windows.Forms.Label]::new()
$label.SetBounds(25,25,510,110)
$label.Text = '正在检查并准备新版程序和技能，请稍候……'
$form.Controls.Add($label)
$start = [Windows.Forms.Button]::new()
$start.SetBounds(270,155,120,38)
$start.Text = '开始升级'
$start.Enabled = $false
$form.Controls.Add($start)
$cancel = [Windows.Forms.Button]::new()
$cancel.SetBounds(410,155,120,38)
$cancel.Text = '稍后'
$form.Controls.Add($cancel)
$cancel.Add_Click({ $form.Close() })
$info = [Diagnostics.ProcessStartInfo]::new()
$info.FileName = Join-Path $PSScriptRoot 'skz.exe'
$info.Arguments = 'update --download'
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$info.StandardOutputEncoding = [Text.Encoding]::UTF8
$info.StandardErrorEncoding = [Text.Encoding]::UTF8
$process = $null
$timer = [Windows.Forms.Timer]::new()
$timer.Interval = 200
$form.Add_Shown({
    try {
        $script:process = [Diagnostics.Process]::Start($info)
        $script:stdout = $process.StandardOutput.ReadToEndAsync()
        $script:stderr = $process.StandardError.ReadToEndAsync()
        $timer.Start()
    } catch { $label.Text = '无法检查新版，请稍后重试。'; $cancel.Text = '关闭' }
})
$timer.Add_Tick({
    if (!$process.HasExited) { return }
    $timer.Stop()
    try {
        if ($process.ExitCode -ne 0) { throw (($stderr.GetAwaiter().GetResult() | ConvertFrom-Json).error.message) }
        $script:release = $stdout.GetAwaiter().GetResult() | ConvertFrom-Json
        if (!$release.available) {
            $label.Text = "当前胜可知 $($release.cli) 无需升级。`r`n正式发布版本：$($release.latest_cli)"
            $cancel.Text = '关闭'
        } else {
            $label.Text = "已准备好胜可知 $($release.latest_cli)，程序与技能将一起升级。`r`n`r`n升级会重新打开 WorkBuddy，请先保存工作。"
            $start.Enabled = $true
        }
    } catch { $label.Text = "检查或下载未完成，原版本未修改。`r`n$($_.Exception.Message)"; $cancel.Text = '关闭' }
})
$start.Add_Click({
    try {
        if ((Get-FileHash -LiteralPath $release.installer -Algorithm SHA256).Hash -ine $release.sha256) { throw '安装包已改变' }
        Start-Process -FilePath $release.installer -ArgumentList '/UPGRADE'
        $form.Close()
    }
    catch { $label.Text = '无法打开升级安装包，请稍后重试。' }
})
$form.Add_FormClosed({
    $timer.Stop()
    if ($process) { if (!$process.HasExited) { $process.Kill() }; $process.Dispose() }
    $timer.Dispose()
})
$null = $form.ShowDialog()

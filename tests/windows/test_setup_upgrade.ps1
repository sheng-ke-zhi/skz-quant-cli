# 在专用交互式 Windows 测试会话运行；使用未发布验收包，结束保留测试版本供检查。
param(
    [Parameter(Mandatory=$true)][string]$BaseInstaller,
    [Parameter(Mandatory=$true)][string]$NewInstaller,
    [Parameter(Mandatory=$true)][string]$FailureInstaller,
    [Parameter(Mandatory=$true)][string]$InterruptedInstaller,
    [string]$BaseVersion='0.1.46', [string]$NewVersion='0.1.47',
    [string]$EvidenceDir="$env:USERPROFILE\skz-upgrade-acceptance"
)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.Encoding]::UTF8
$install="$env:LOCALAPPDATA\Programs\SKZ"
$cli=Join-Path $install 'skz.exe'
$state="$env:LOCALAPPDATA\SKZ\Installer"
$cases=[Collections.Generic.List[object]]::new()
$null=New-Item -ItemType Directory -Path $EvidenceDir -Force
$baseline=Get-FileHash "$env:LOCALAPPDATA\skz\credentials" -Algorithm SHA256

function Save-Progress([string]$Name) {
    @{case=$Name;completed=@($cases.ToArray())} | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $EvidenceDir 'progress.json') -Encoding UTF8
}
function Assert-State([string]$Expected,[int]$Skills) {
    $version=& $cli --version | ConvertFrom-Json
    if($LASTEXITCODE -ne 0 -or $version.cli -ne $Expected){throw "Expected program $Expected"}
    $status=& $cli plugin status workbuddy | ConvertFrom-Json
    if($LASTEXITCODE -ne 0 -or !$status.installed -or !$status.content_ok -or !$status.native_ok -or $status.needs_upgrade){throw 'Program and native skills differ'}
    $manifest=Get-Content "$install\plugins\manifest.json" -Raw -Encoding UTF8 | ConvertFrom-Json
    $registry=Get-Content "$env:USERPROFILE\.workbuddy\plugins\installed_plugins.json" -Raw -Encoding UTF8 | ConvertFrom-Json
    $entry=@($registry.plugins.'skz@skz' | Where-Object {$_.scope -eq 'user'})
    $actual=@(Get-ChildItem (Join-Path $entry[0].installPath 'skills') -Directory | Select-Object -ExpandProperty Name)
    if($manifest.cli -ne $Expected -or $actual.Count -ne $Skills -or (Compare-Object $actual @($manifest.skills))){throw 'Native skill set does not match package'}
    $auth=& $cli auth status | ConvertFrom-Json
    if($LASTEXITCODE -ne 0 -or !$auth.present -or !$auth.readOnly -or $auth.active -ne 'default'){throw 'Login or read-only policy changed'}
    if((Get-FileHash $baseline.Path -Algorithm SHA256).Hash -ne $baseline.Hash){throw 'SKZ credentials changed during installation'}
    return @{version=$version.cli;skills=$actual.Count;credentialsUnchanged=$true;readOnly=$true;exeSHA256=(Get-FileHash $cli -Algorithm SHA256).Hash}
}
function Run-Installer([string]$Path,[int]$ExpectedCode=0) {
    $process=Start-Process -FilePath $Path -ArgumentList '/S' -PassThru
    if(!$process.WaitForExit(300000)){throw 'Installer timed out; preserve journal for recovery'}
    if($process.ExitCode -ne $ExpectedCode){throw "Unexpected installer exit $($process.ExitCode), expected $ExpectedCode"}
}
function Record([string]$Name,$Result) {
    $cases.Add(@{name=$Name;passed=$true;result=$Result})
    Copy-Item "$state\last.log" (Join-Path $EvidenceDir "$Name.log") -Force
    Save-Progress $Name
}

try {
    Save-Progress 'baseline'
    Run-Installer $BaseInstaller
    $before=Assert-State $BaseVersion 10
    Record 'baseline' $before

    Save-Progress 'failure-after-native-update'
    Run-Installer $FailureInstaller 1
    $result=Get-Content "$state\result.json" -Raw -Encoding UTF8 | ConvertFrom-Json
    if($result.outcome -ne 'restored'){throw 'Failed update did not restore the old version'}
    $text=Get-Content "$state\last.log" -Raw -Encoding UTF8
    if($text -notmatch "$NewVersion，11 个技能"){throw 'Fault did not occur after the actual new skill validation'}
    $restored=Assert-State $BaseVersion 10
    if($restored.exeSHA256 -ne $before.exeSHA256){throw 'Program rollback hash differs'}
    Record 'failure-after-native-update' $restored

    Save-Progress 'interrupted-update'
    $ready=Join-Path $EvidenceDir 'hold-ready.json'
    Remove-Item $ready -Force -ErrorAction SilentlyContinue
    $process=Start-Process -FilePath $InterruptedInstaller -ArgumentList '/S' -PassThru
    $deadline=(Get-Date).AddSeconds(180)
    while(!(Test-Path $ready)) {
        if($process.HasExited -or (Get-Date) -ge $deadline){throw 'Interrupted fixture never reached the native validation point'}
        Start-Sleep -Milliseconds 300
    }
    $holding=Get-Content $ready -Raw -Encoding UTF8 | ConvertFrom-Json
    $nativeNew=Assert-State $NewVersion 11
    $all=@(Get-CimInstance Win32_Process)
    $cursor=[int]$holding.pid
    $found=$false
    for($i=0;$i -lt 8;$i++) {
        if($cursor -eq $process.Id){$found=$true;break}
        $cursor=[int]($all | Where-Object {$_.ProcessId -eq $cursor} | Select-Object -First 1).ParentProcessId
    }
    if(!$found){throw 'Refusing to stop a process outside this test installer'}
    Stop-Process -Id $holding.pid -Force
    if(!$process.WaitForExit(30000)){throw 'Interrupted installer did not exit'}
    if(!(Test-Path "$state\transaction.json")){throw 'Interrupted update did not leave its recovery journal'}
    Run-Installer $BaseInstaller
    $restored=Assert-State $BaseVersion 10
    if(Test-Path "$state\transaction.json"){throw 'Recovery journal was not cleared'}
    Record 'interrupted-update' @{interrupted=$nativeNew;restored=$restored}

    Save-Progress 'successful-cross-version-update'
    Run-Installer $NewInstaller
    $new=Assert-State $NewVersion 11
    if($new.exeSHA256 -eq $before.exeSHA256){throw 'Program bytes did not change'}
    Record 'successful-cross-version-update' $new

    Save-Progress 'older-package-refused'
    Run-Installer $BaseInstaller 1
    $unchanged=Assert-State $NewVersion 11
    if($unchanged.exeSHA256 -ne $new.exeSHA256){throw 'Older installer modified the newer program'}
    Record 'older-package-refused' $unchanged

    Save-Progress 'same-version-idempotency'
    Run-Installer $NewInstaller
    Record 'same-version-idempotency' (Assert-State $NewVersion 11)

    Save-Progress 'disabled-plugin-repair'
    $null=& $cli __workbuddy-setup disable
    if($LASTEXITCODE -ne 0){throw 'Unable to prepare disabled plugin fixture'}
    $registry=Get-Content "$env:USERPROFILE\.workbuddy\plugins\installed_plugins.json" -Raw -Encoding UTF8 | ConvertFrom-Json
    $entry=@($registry.plugins.'skz@skz' | Where-Object {$_.scope -eq 'user'})[0]
    [IO.File]::WriteAllText((Join-Path $entry.installPath 'skills\skz-guide\SKILL.md'),'acceptance damage')
    Run-Installer $NewInstaller
    $fixed=Assert-State $NewVersion 11
    $disabled=& $cli __workbuddy-setup check | ConvertFrom-Json
    if($disabled.enabled -ne $false){throw 'Repair enabled a previously disabled plugin'}
    Record 'disabled-plugin-repair' $fixed

    Save-Progress 'complete'
} catch {
    @{case='failed';message=$_.Exception.Message;completed=@($cases.ToArray())} | ConvertTo-Json -Depth 8 |
        Set-Content (Join-Path $EvidenceDir 'progress.json') -Encoding UTF8
    throw
}

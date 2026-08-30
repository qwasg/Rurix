# G37 W1 rurixc「if 包 while」修复:生产 kernel codegen 零漂移对拍脚本。
# 用法:.\drift_check.ps1 -OutDir <dir>  (在仓根运行;当前 target\debug\rurixc.exe 编译
# src/rurix-render/kernels + src/rurix-rt/kernels 全部 .rx → <dir>,并写 manifest.json:
# 逐文件 rc + SPV SHA256。pre/post 两态各跑一次后由 compare 段比对。)
param([Parameter(Mandatory)][string]$OutDir)

$ErrorActionPreference = "Continue"
New-Item -ItemType Directory -Force $OutDir | Out-Null
$exe = "target\debug\rurixc.exe"
$manifest = @{}
$files = @(Get-ChildItem "src\rurix-render\kernels\*.rx") + @(Get-ChildItem "src\rurix-rt\kernels\*.rx")
foreach ($f in $files) {
    $key = $f.Directory.Parent.Name + "/" + $f.Name
    $spv = Join-Path $OutDir ($f.Directory.Parent.Name + "__" + $f.BaseName + ".spv")
    cmd /c "$exe --target vulkan `"$($f.FullName)`" -o `"$spv`" >nul 2>nul"
    $rc = $LASTEXITCODE
    if ($rc -eq 0 -and (Test-Path $spv)) {
        $h = (Get-FileHash $spv -Algorithm SHA256).Hash
        $manifest[$key] = @{ rc = 0; sha256 = $h }
    } else {
        $manifest[$key] = @{ rc = $rc; sha256 = $null }
    }
}
$manifest | ConvertTo-Json | Set-Content (Join-Path $OutDir "manifest.json") -Encoding UTF8
Write-Host ("{0}: {1} kernels, {2} compiled ok" -f $OutDir, $files.Count, (@($manifest.Values | Where-Object { $_.rc -eq 0 }).Count))

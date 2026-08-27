# build.ps1 — Rurix 渲染器 C ABI 最小宿主示例一键构建+真跑（G31+ 波 C Task C2）。
# 用法（仓库任意目录）：
#   powershell -ExecutionPolicy Bypass -File docs\renderer\examples\minimal_host\build.ps1
# 产物落 docs/renderer/examples/minimal_host/build/（rurix_rhi.dll/.lib/.h + minimal_host.exe）。
# 三态：缺 rurixc/clang/MSVC → 打印 DEV_ENV_DEGRADE 退出 0（不冒充 PASS）；
#   RURIX_REQUIRE_REAL=1 下降级翻硬 FAIL 退出 1。
#Requires -Version 5.1
param()
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = (Resolve-Path (Join-Path $ScriptDir "..\..\..\..")).Path
$Build = Join-Path $ScriptDir "build"
$RequireReal = $env:RURIX_REQUIRE_REAL -eq "1"

function Degrade([string]$reason) {
    if ($RequireReal) { Write-Host "[minimal_host] FAIL DEV_ENV_DEGRADE: $reason (RURIX_REQUIRE_REAL=1)"; exit 1 }
    Write-Host "[minimal_host] SKIP DEV_ENV_DEGRADE: $reason"; exit 0
}

New-Item -ItemType Directory -Force -Path $Build | Out-Null

# ── 步骤 0 · 工具链定位 ─────────────────────────────────────────────────────
$Rurixc = Join-Path $Root "target\debug\rurixc.exe"
if (-not (Test-Path $Rurixc)) {
    Write-Host "[minimal_host] rurixc 缺失，构建中（cargo build -p rurixc）…"
    cargo build -p rurixc --quiet 2>&1 | Out-Null
    if (-not (Test-Path $Rurixc)) { Degrade "rurixc 构建失败（cargo 不在 PATH？）" }
}

$Clang = $env:RURIXC_CLANG
if (-not $Clang) { $Clang = "C:\Program Files\LLVM\bin\clang.exe" }
if (-not (Test-Path $Clang)) { Degrade "clang 22.1.x 缺失（装 LLVM 或设 RURIXC_CLANG）" }
$env:RURIXC_CLANG = $Clang

$MsvcRoot = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207"
if (-not (Test-Path $MsvcRoot)) {
    $cand = Get-ChildItem "C:\Program Files\Microsoft Visual Studio\2022\*\VC\Tools\MSVC" -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending | Select-Object -First 1
    if ($cand) { $MsvcRoot = $cand.FullName } else { Degrade "MSVC 2022 缺失" }
}
$ClExe = Join-Path $MsvcRoot "bin\Hostx64\x64\cl.exe"
$SdkInc = "C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0"
$SdkLib = "C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0"
if (-not (Test-Path $SdkInc)) {
    $cand = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\Include" -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending | Select-Object -First 1
    if ($cand) { $SdkInc = $cand.FullName; $SdkLib = $SdkInc -replace '\\Include\\', '\Lib\' } else { Degrade "Windows SDK 缺失" }
}
if (-not (Test-Path $ClExe)) { Degrade "cl.exe 缺失（$ClExe）" }

# ── 步骤 1 · .rx 导出面 → rurix_rhi.dll + import lib + 生成头 ───────────────
$Stem = Join-Path $Build "rurix_rhi"
Write-Host "[minimal_host] rurixc --emit=dll apps/uc05-rhi/src/embed.rx …"
& $Rurixc (Join-Path $Root "apps\uc05-rhi\src\embed.rx") --emit=dll -o $Stem
if ($LASTEXITCODE -ne 0) { Degrade "rurixc --emit=dll 失败 rc=$LASTEXITCODE（link.exe/rt_cabi 面）" }
foreach ($ext in @(".dll", ".lib", ".h")) {
    if (-not (Test-Path ($Stem + $ext))) { Degrade "emit 产物缺 rurix_rhi$ext" }
}

# ── 步骤 2 · cl.exe 编译宿主（生成头目录进 INCLUDE；链 import lib）─────────
$env:INCLUDE = @(
    (Join-Path $MsvcRoot "include"),
    (Join-Path $SdkInc "ucrt"),
    (Join-Path $SdkInc "shared"),
    (Join-Path $SdkInc "um"),
    $Build
) -join ";"
$env:LIB = @(
    (Join-Path $MsvcRoot "lib\x64"),
    (Join-Path $SdkLib "ucrt\x64"),
    (Join-Path $SdkLib "um\x64"),
    $Build
) -join ";"
$env:PATH = (Join-Path $MsvcRoot "bin\Hostx64\x64") + ";" + $env:PATH

$HostExe = Join-Path $Build "minimal_host.exe"
Write-Host "[minimal_host] cl.exe 编译 minimal_host.cpp …"
& $ClExe /std:c++17 /EHsc /nologo (Join-Path $ScriptDir "minimal_host.cpp") `
    /Fe:$HostExe /link /LIBPATH:$Build rurix_rhi.lib 2>&1 | Write-Host
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $HostExe)) {
    Write-Host "[minimal_host] FAIL 宿主编译失败 rc=$LASTEXITCODE"; exit 1
}

# ── 步骤 3 · 真跑（需 Vulkan 可用 GPU；rurix_rhi.dll 同目录）────────────────
Write-Host "[minimal_host] 真跑 minimal_host.exe …"
$out = & $HostExe 2>&1
$out | Write-Host
if ($LASTEXITCODE -ne 0) {
    if (($out -join " ") -match "skipped_dev_env|Vulkan|vkCreateInstance") {
        Degrade "真跑 dev-env 降级（无 Vulkan/GPU）"
    }
    Write-Host "[minimal_host] FAIL 真跑 rc=$LASTEXITCODE"; exit 1
}
if (($out -join "`n") -notmatch "RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000") {
    Write-Host "[minimal_host] FAIL 输出标记不符（预期 RURIX_MINIMAL_HOST_OK … pixel=0x00000000）"; exit 1
}
Write-Host "[minimal_host] PASS 五步最小集成走通（产物：$Build）"
exit 0

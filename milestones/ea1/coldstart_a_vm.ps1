# Rurix cold-start segment A (vm_rxcheck) one-shot measurement.
# Run INSIDE a clean Win11 VM (no Rust/LLVM/VS preinstalled; network required).
# One-liner to fetch+run inside the VM:
#   powershell -ep Bypass -c "curl.exe -sL -o a.ps1 https://raw.githubusercontent.com/qwasg/Rurix/main/milestones/ea1/coldstart_a_vm.ps1; ./a.ps1"
# Protocol: RFC-0012 4.10 / RXS-0219, ruling C, segment A:
#   T0 = first documented command (downloading rurixup.exe) -- download IS counted;
#   T1 = `rx check hello_kernel.rx` exit 0 (pure front-end; zero GPU / zero MSVC).
# Criterion: T1 - T0 <= 600 s. All attempts are honest data; do not re-run to shop.
param([string]$Ver = "v1.0.1-dist.2", [int]$Attempt = 1)
$ErrorActionPreference = "Continue"
$work = Join-Path $env:USERPROFILE "rurix-coldstart"
New-Item -ItemType Directory -Force $work | Out-Null
Set-Location $work
$stamp = [DateTime]::UtcNow.ToString("yyyyMMdd")
$desk = [Environment]::GetFolderPath("Desktop")
$log = Join-Path $desk "rurix_coldstart_a_${stamp}_a$Attempt.log"
$out = Join-Path $desk "rurix_coldstart_a_${stamp}_a$Attempt.json"
Start-Transcript -Path $log -Force | Out-Null

function NowIso { [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ") }
$steps = @()

Write-Output ("user=" + $env:USERNAME + " host=" + $env:COMPUTERNAME)
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
$rustc = Get-Command rustc -ErrorAction SilentlyContinue
$clang = Get-Command clang -ErrorAction SilentlyContinue
Write-Output ("cleanliness: cargo=" + [bool]$cargo + " rustc=" + [bool]$rustc + " clang=" + [bool]$clang + " (expect all False in a clean VM)")

# --- T0: first documented command = bootstrap download (COUNTED in segment A) ---
$t0 = NowIso
$t = [Diagnostics.Stopwatch]::StartNew()
curl.exe --fail --silent --show-error --location --proto =https --proto-redir =https -o rurixup.exe "https://github.com/qwasg/Rurix/releases/download/$Ver/rurixup.exe"
$dlExit = $LASTEXITCODE; $t.Stop()
$dlBytes = 0; if (Test-Path rurixup.exe) { $dlBytes = (Get-Item rurixup.exe).Length }
$steps += @{ name = "bootstrap_download_rurixup(T0,counted)"; cmd = "curl.exe --proto =https ... rurixup.exe"; exit = $dlExit; duration_s = [Math]::Round($t.Elapsed.TotalSeconds, 2); note = "$dlBytes bytes" }
Write-Output ("bootstrap: exit=" + $dlExit + " bytes=" + $dlBytes)
if ($dlExit -ne 0) { Write-Output "ABORT: bootstrap failed"; Stop-Transcript | Out-Null; exit 1 }

# --- install via raw anchor (four-level content-addressed verification) ---
$t = [Diagnostics.Stopwatch]::StartNew()
$installOut = & .\rurixup.exe install $Ver --channel-file "https://raw.githubusercontent.com/qwasg/Rurix/main/channels/stable.json" --registry (Join-Path $env:USERPROFILE ".rurix\toolchains.json") 2>&1 | Out-String
$instExit = $LASTEXITCODE; $t.Stop()
Write-Output $installOut
$steps += @{ name = "install_via_anchor"; cmd = "rurixup.exe install $Ver --channel-file <raw anchor url>"; exit = $instExit; duration_s = [Math]::Round($t.Elapsed.TotalSeconds, 2); note = ($installOut -split "`n" | Select-String "RURIXUP_INSTALL" | Select-Object -First 1 | Out-String).Trim() }
if ($instExit -ne 0) { Write-Output "install FAILED (honest data - keep this log)" }

# --- hello_kernel.rx: kernel typecheck sample (check-only; zero GPU needed) ---
$sample = @'
kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>,
                a: f32, n: usize, t: ThreadCtx<1>) {
    let i = t.global_id();
    if i < n {
        out[i] = a * x[i] + out[i];
    }
}

fn main() {
    println("hello_kernel: static check only");
}
'@
[IO.File]::WriteAllText((Join-Path $work "hello_kernel.rx"), $sample.Replace("`r`n", "`n"))

$rx = Join-Path $env:USERPROFILE ".rurix\toolchains\$Ver\bin\rx.exe"
Write-Output ("rx = " + $rx + " exists=" + (Test-Path $rx))
$t = [Diagnostics.Stopwatch]::StartNew()
$checkOut = & $rx check (Join-Path $work "hello_kernel.rx") 2>&1 | Out-String
$checkExit = $LASTEXITCODE; $t.Stop()
Write-Output $checkOut
$t1 = NowIso
$steps += @{ name = "rx_check_hello_kernel"; cmd = "rx.exe check hello_kernel.rx (pure front-end; zero GPU/MSVC)"; exit = $checkExit; duration_s = [Math]::Round($t.Elapsed.TotalSeconds, 2); note = "T1 = exit 0" }

# --- evidence json (schema: milestones/ea1/install_e2e_evidence_schema.json) ---
$os = (Get-CimInstance Win32_OperatingSystem)
$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name
$durTotal = ([DateTime]::Parse($t1.Replace("Z","+00:00")) - [DateTime]::Parse($t0.Replace("Z","+00:00"))).TotalSeconds
$digest = 0; if ($installOut -match "digest_levels_verified=(\d)") { $digest = [int]$Matches[1] }
$doc = [ordered]@{
  segment = "vm_rxcheck"
  host = [ordered]@{ os = ($os.Caption + " build " + $os.BuildNumber + " (VM)"); cpu = $cpu; gpu = "none (consumer VM, no GPU passthrough - per ruling C this segment ends at rx check)"; driver = "n/a" }
  toolchain_version = $Ver
  t_start = $t0
  t_end = $t1
  duration_s = [Math]::Round($durTotal, 2)
  steps = $steps
  digest_levels_verified = $digest
  bytes_downloaded = $dlBytes
  bandwidth_note = "VM NAT over home broadband; download counted in T0..T1 per segment-A protocol"
  attempt = $Attempt
  pass = (($dlExit -eq 0) -and ($instExit -eq 0) -and ($checkExit -eq 0) -and ($durTotal -le 600))
  notes = ("clean-VM run: user=" + $env:USERNAME + "; cargo/rustc/clang present=" + [bool]$cargo + "/" + [bool]$rustc + "/" + [bool]$clang + "; VM CPU scheduling noise acceptable (criterion 600 s)")
}
$doc | ConvertTo-Json -Depth 6 | Out-File -Encoding utf8 $out
Write-Output ("RESULT pass=" + $doc.pass + " duration_s=" + $doc.duration_s)
Write-Output ("Evidence JSON on your Desktop: " + $out)
Write-Output "Copy the .json (and .log) back to the host and hand them to the agent."
Stop-Transcript | Out-Null

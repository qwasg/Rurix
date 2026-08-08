$env:RURIX_REQUIRE_REAL = '1'
$env:RURIX_VK_VALIDATION = '1'
foreach ($a in @('--m19-none', '--m19-red-stale', '--m19-red-missing-local')) {
    $args2 = @('run', '-q', '-p', 'uc06-renderer', '--features', 'vulkan', '--', '--m19-vsm-page-cache')
    if ($a -ne '--m19-none') { $args2 += $a }
    $out = & cargo @args2 2>$null
    $line = $out | Select-String '"subject"' | Select-Object -First 1
    if ($null -eq $line) { "[$a] NO JSON"; continue }
    $j = $line.ToString() | ConvertFrom-Json
    "[$a] pt=$($j.page_table_digest_frames_matched) pool=$($j.depth_pool_digest_frames_matched) smp=$($j.sample_digest_frames_matched) mism=$($j.sample_value_mismatches) shadowed=$($j.device_samples_shadowed)/$($j.device_samples_total) depth=$($j.depth_match) red_ok=$($j.red_ok)"
}

#requires -Version 5.1
<#
.SYNOPSIS
  task-011 冒烟：实跑 5 个 scan_*.ps1，stdout 落临时文件（字节级），逐行校验 NDJSON schema。
  （PS 的 *> 会把原生输出包成 UTF-16，故用 cmd /c 重定向对齐 Rust 按字节读 stdout 的方式。）
#>

$ErrorActionPreference = 'Stop'
$scripts = @(
    @{ file = 'scan_temp.ps1';           module = 'temp' }
    @{ file = 'scan_windows_update.ps1'; module = 'windows_update' }
    @{ file = 'scan_prefetch.ps1';       module = 'prefetch' }
    @{ file = 'scan_browser_cache.ps1';  module = 'browser_cache' }
    @{ file = 'scan_dev_cache.ps1';      module = 'dev_cache' }
)

$fail = 0
foreach ($s in $scripts) {
    $path = Join-Path $PSScriptRoot $s.file
    $tmp = [System.IO.Path]::GetTempFileName()
    $err = [System.IO.Path]::GetTempFileName()
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'powershell.exe'
    $psi.Arguments = '-NoProfile -ExecutionPolicy Bypass -File "' + $path + '"'
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $p = [System.Diagnostics.Process]::Start($psi)
    $out = $p.StandardOutput.ReadToEnd()
    $p.WaitForExit()
    [System.IO.File]::WriteAllText($tmp, $out, [System.Text.UTF8Encoding]::new($false))

    $raw = [string]$out
    $raw = $raw.TrimStart([char]0xFEFF)
    $lines = @($raw -split "`r?`n" | ForEach-Object { ([string]$_ -replace '[^\x20-\x7E]', '') } | Where-Object { $_ -ne '' })
    $itemCount = 0
    try {
        if ($lines.Count -eq 0) { throw "no output" }
        for ($i = 0; $i -lt $lines.Count; $i++) {
            $obj = $null
            try {
                $obj = $lines[$i] | ConvertFrom-Json
            } catch {
                if ($lines[$i] -match '"type"\s*:\s*"(\w+)"') {
                    $obj = [PSCustomObject]@{ type = $Matches[1] }
                } else {
                    throw "cannot parse line"
                }
            }
            if ($i -eq $lines.Count - 1) {
                if ($obj.type -ne 'done') { throw "last line not done" }
                if ($obj.module_code -ne $s.module) { throw "done module mismatch" }
                if ($obj.total_items -ne $itemCount) { throw "total_items mismatch: $itemCount vs $($obj.total_items)" }
            } else {
                if ($obj.type -ne 'item') { throw "non-last line not item" }
                foreach ($f in 'module_code','path','size_bytes','risk_level') {
                    if ($null -eq $obj.$f) { throw "missing field $f" }
                }
                if ($obj.module_code -ne $s.module) { throw "item module mismatch" }
                $itemCount++
            }
        }
        Write-Host ("[OK] {0}: items={1}, done={2}" -f $s.file, $itemCount, ($lines[-1]))
    } catch {
        $fail++
        Write-Host ("[FAIL] {0}: {1}" -f $s.file, $_)
    }
    Remove-Item $tmp, $err -Force -ErrorAction SilentlyContinue
}
if ($fail -gt 0) { exit 1 }

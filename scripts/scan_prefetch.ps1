#requires -Version 5.1
<#
.SYNOPSIS
  扫描 C:\Windows\Prefetch，逐行输出 NDJSON item，最后 done。不删除；权限不足写 stderr。
.MODULE_CODE
  prefetch
#>

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$moduleCode = 'prefetch'
$script:totalItems = 0
$script:totalBytes = [int64]0

function Write-Item([string]$category, [string]$path, [int64]$size, [int]$risk) {
    $obj = [ordered]@{
        type        = 'item'
        module_code = $moduleCode
        category    = $category
        path        = $path
        size_bytes  = $size
        risk_level  = $risk
    }
    [Console]::Out.WriteLine(($obj | ConvertTo-Json -Compress))
    $script:totalItems++
    $script:totalBytes += $size
}

# v0.2.1 Bug-2：风险等级不再是写死的 1（判据同 scan_browser_cache.ps1）。
# Prefetch 是预读取缓存，Windows 会自动重建；删掉只会让常用程序「下次启动
# 稍慢一点」，属安全缓存 → L1。
$riskPrefetch = 1

$root = 'C:\Windows\Prefetch'
if (Test-Path -LiteralPath $root) {
    Get-ChildItem -LiteralPath $root -Recurse -Force -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            if ($_.LinkType) { return }
            try { Write-Item 'prefetch' $_.FullName $_.Length $riskPrefetch }
            catch { [Console]::Error.WriteLine("skip $($_.FullName): $_") }
        }
} else {
    [Console]::Error.WriteLine("root not found: $root")
}

$done = [ordered]@{
    type          = 'done'
    module_code   = $moduleCode
    total_items   = $script:totalItems
    total_bytes   = $script:totalBytes
}
[Console]::Out.WriteLine(($done | ConvertTo-Json -Compress))

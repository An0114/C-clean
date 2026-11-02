#requires -Version 5.1
<#
.SYNOPSIS
  扫描开发者缓存：npm / pip / pnpm store，逐行输出 NDJSON item，最后 done。
  不删除；目录不存在则跳过；权限不足写 stderr。
.MODULE_CODE
  dev_cache
#>

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$moduleCode = 'dev_cache'
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
# 包管理器缓存都是「删了下次安装重新下载」，一律 L1。
$riskCache = 1

$roots = @(
    @{ path = (Join-Path $env:LOCALAPPDATA 'npm-cache'); category = 'npm_cache'; risk = $riskCache }
    @{ path = (Join-Path $env:APPDATA 'npm-cache'); category = 'npm_cache'; risk = $riskCache }
    @{ path = (Join-Path $env:LOCALAPPDATA 'pip\cache'); category = 'pip_cache'; risk = $riskCache }
    @{ path = (Join-Path $env:LOCALAPPDATA 'pnpm-store'); category = 'pnpm_store'; risk = $riskCache }
)

foreach ($r in $roots) {
    if (-not (Test-Path -LiteralPath $r.path)) {
        [Console]::Error.WriteLine("skip missing: $($r.path)")
        continue
    }
    Get-ChildItem -LiteralPath $r.path -Recurse -Force -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            if ($_.LinkType) { return }
            try { Write-Item $r.category $_.FullName $_.Length $r.risk }
            catch { [Console]::Error.WriteLine("skip $($_.FullName): $_") }
        }
}

$done = [ordered]@{
    type          = 'done'
    module_code   = $moduleCode
    total_items   = $script:totalItems
    total_bytes   = $script:totalBytes
}
[Console]::Out.WriteLine(($done | ConvertTo-Json -Compress))

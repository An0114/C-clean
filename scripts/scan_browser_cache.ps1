#requires -Version 5.1
<#
.SYNOPSIS
  扫描 Edge / Chrome 浏览器缓存目录，逐行输出 NDJSON item，最后 done。
  不删除；目录不存在则跳过不报错；权限不足写 stderr。
  注意：只扫 ...\User Data\Default\Cache 缓存子目录，不碰 Login Data / Bookmarks 等 L3 Profile 文件。
.MODULE_CODE
  browser_cache
#>

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$moduleCode = 'browser_cache'
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

$roots = @(
    @{ path = (Join-Path $env:LOCALAPPDATA 'Microsoft\Edge\User Data\Default\Cache'); category = 'edge_cache' }
    @{ path = (Join-Path $env:LOCALAPPDATA 'Google\Chrome\User Data\Default\Cache'); category = 'chrome_cache' }
)

foreach ($r in $roots) {
    if (-not (Test-Path -LiteralPath $r.path)) {
        [Console]::Error.WriteLine("skip missing: $($r.path)")
        continue
    }
    Get-ChildItem -LiteralPath $r.path -Recurse -Force -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            if ($_.LinkType) { return }
            try { Write-Item $r.category $_.FullName $_.Length 1 }
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

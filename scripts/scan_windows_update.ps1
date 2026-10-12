#requires -Version 5.1
<#
.SYNOPSIS
  扫描 Windows 更新下载缓存（SoftwareDistribution\Download），逐行输出 NDJSON item，最后 done。
  不删除；权限不足写 stderr 并跳过。
.MODULE_CODE
  windows_update
#>

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$moduleCode = 'windows_update'
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

$root = 'C:\Windows\SoftwareDistribution\Download'
if (Test-Path -LiteralPath $root) {
    Get-ChildItem -LiteralPath $root -Recurse -Force -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            if ($_.LinkType) { return }
            try { Write-Item 'update_download' $_.FullName $_.Length 1 }
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

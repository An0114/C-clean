#requires -Version 5.1
<#
.SYNOPSIS
  扫描用户临时目录 + 系统临时目录，逐行输出 NDJSON item 事件，最后输出 done 事件。
  不删除任何文件；跳过符号链接/junction；权限不足目录写 stderr 并跳过。
.MODULE_CODE
  temp
#>

[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$moduleCode = 'temp'
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

function Scan-Dir([string]$root, [string]$category, [int]$risk) {
    if (-not (Test-Path -LiteralPath $root)) { return }
    Get-ChildItem -LiteralPath $root -Recurse -Force -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            # 跳过符号链接/junction（对齐 ARCHITECTURE R6）：文件本身 LinkType 非空则跳过
            if ($_.LinkType) { return }
            try { Write-Item $category $_.FullName $_.Length $risk } catch { [Console]::Error.WriteLine("skip $($_.FullName): $_") }
        }
}

# v0.2.1 Bug-2：风险等级不再是写死的 1，按目录分别给。
#
# - `%TEMP%`（用户临时目录）：删了只影响自己，程序会重建 → **L1**
# - `C:\Windows\Temp`（系统临时目录）：里面多数文件属于 SYSTEM，普通权限删不掉；
#   而且安装程序/系统服务正在使用它，删错可能让正在进行的安装中断 → **L2**（需确认）
Scan-Dir $env:TEMP 'user_temp' 1
Scan-Dir 'C:\Windows\Temp' 'windows_temp' 2

$done = [ordered]@{
    type          = 'done'
    module_code   = $moduleCode
    total_items   = $script:totalItems
    total_bytes   = $script:totalBytes
}
[Console]::Out.WriteLine(($done | ConvertTo-Json -Compress))

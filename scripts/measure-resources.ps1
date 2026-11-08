<#
.SYNOPSIS
Read-only resource sample for one Wordglance instance and its WebView2 descendants.
.EXAMPLE
.\scripts\measure-resources.ps1 -SampleSeconds 10 -OutputJson docs\performance-baseline.json
.EXAMPLE
.\scripts\measure-resources.ps1 -RootProcessId 1234 -SampleSeconds 30
.NOTES
Windows PowerShell 5.1 or PowerShell 7 on Windows. No command lines, window
contents, credentials, or environment variables are read. The application is
never started, stopped, or sent input.
#>
[CmdletBinding()]
param(
    [Alias('RootPid')]
    [ValidateRange(1, 2147483647)]
    [int]$RootProcessId,

    [ValidateRange(1, 3600)]
    [double]$SampleSeconds = 10,

    [string]$OutputJson
)

$ErrorActionPreference = 'Stop'

function Read-ProcessTable {
    # Explicit properties: never retrieve CommandLine (which can contain secrets).
    @(Get-CimInstance -Query @'
SELECT ProcessId, ParentProcessId, Name, CreationDate,
       WorkingSetSize, PrivatePageCount, KernelModeTime, UserModeTime
FROM Win32_Process
'@)
}

function Select-AppProcesses {
    param([object[]]$Table, [int]$AppProcessId, [datetime]$AppCreatedAt)

    $byId = @{}
    $children = @{}
    foreach ($entry in $Table) {
        $byId[[int]$entry.ProcessId] = $entry
        $parentId = [int]$entry.ParentProcessId
        if (-not $children.ContainsKey($parentId)) {
            $children[$parentId] = [System.Collections.Generic.List[object]]::new()
        }
        $children[$parentId].Add($entry)
    }

    $root = $byId[$AppProcessId]
    if ($null -eq $root -or $root.Name -ine 'wordglance.exe' -or
        $root.CreationDate -ne $AppCreatedAt) {
        throw "Wordglance PID $AppProcessId exited or changed identity during sampling. Run a new sample."
    }

    $pending = [System.Collections.Generic.Stack[object]]::new()
    $pending.Push($root)
    $visited = @{}
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $currentId = [int]$current.ProcessId
        if ($visited.ContainsKey($currentId)) { continue }
        $visited[$currentId] = $true
        if ($currentId -eq $AppProcessId -or $current.Name -ieq 'msedgewebview2.exe') {
            $current
        }
        foreach ($child in $children[$currentId]) {
            # A reused parent PID must not attach an older, unrelated process.
            if ($null -ne $child -and $child.CreationDate -ge $current.CreationDate) {
                $pending.Push($child)
            }
        }
    }
}

$table = Read-ProcessTable
if (-not $PSBoundParameters.ContainsKey('RootProcessId')) {
    $roots = @($table | Where-Object Name -IEQ 'wordglance.exe')
    if ($roots.Count -ne 1) {
        throw "Found $($roots.Count) Wordglance instances. Supply -RootProcessId for exactly one running wordglance.exe."
    }
    $RootProcessId = [int]$roots[0].ProcessId
}
$root = $table | Where-Object ProcessId -EQ $RootProcessId | Select-Object -First 1
if ($null -eq $root -or $root.Name -ine 'wordglance.exe') {
    throw "PID $RootProcessId is not a running wordglance.exe."
}

$rootCreatedAt = [datetime]$root.CreationDate
$logicalProcessors = [int](Get-CimInstance Win32_ComputerSystem -Property NumberOfLogicalProcessors).NumberOfLogicalProcessors
$current = @(Select-AppProcesses $table $RootProcessId $rootCreatedAt)
$startedAt = [DateTimeOffset]::UtcNow
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$lastCpu = @{}
$observed = @{}
$cpuSeconds = 0.0
$sampleCount = 1

foreach ($entry in $current) {
    $key = '{0}:{1}' -f $entry.ProcessId, $entry.CreationDate.ToUniversalTime().Ticks
    $lastCpu[$key] = ([double]$entry.KernelModeTime + [double]$entry.UserModeTime) / 10000000.0
    $observed[$key] = $true
}

do {
    $remaining = $SampleSeconds - $timer.Elapsed.TotalSeconds
    if ($remaining -gt 0) {
        Start-Sleep -Milliseconds ([int][Math]::Ceiling([Math]::Min(1.0, $remaining) * 1000))
    }
    $current = @(Select-AppProcesses (Read-ProcessTable) $RootProcessId $rootCreatedAt)
    foreach ($entry in $current) {
        $key = '{0}:{1}' -f $entry.ProcessId, $entry.CreationDate.ToUniversalTime().Ticks
        $totalCpu = ([double]$entry.KernelModeTime + [double]$entry.UserModeTime) / 10000000.0
        if ($lastCpu.ContainsKey($key)) {
            $cpuSeconds += [Math]::Max(0, $totalCpu - $lastCpu[$key])
        } elseif ($entry.CreationDate.ToUniversalTime() -ge $startedAt.UtcDateTime) {
            $cpuSeconds += $totalCpu
        }
        $lastCpu[$key] = $totalCpu
        $observed[$key] = $true
    }
    $sampleCount++
} while ($timer.Elapsed.TotalSeconds -lt $SampleSeconds)

$timer.Stop()
$endedAt = [DateTimeOffset]::UtcNow
$workingSetBytes = ($current | Measure-Object WorkingSetSize -Sum).Sum
$privateBytes = ($current | Measure-Object PrivatePageCount -Sum).Sum
$processDetails = @($current | Sort-Object ProcessId | ForEach-Object {
    [pscustomobject][ordered]@{
        ProcessId = [int]$_.ProcessId
        ParentProcessId = [int]$_.ParentProcessId
        Name = $_.Name
        CreatedAtUtc = $_.CreationDate.ToUniversalTime().ToString('o')
        WorkingSetMiB = [Math]::Round([double]$_.WorkingSetSize / 1MB, 3)
        PrivateBytesMiB = [Math]::Round([double]$_.PrivatePageCount / 1MB, 3)
    }
})

$result = [pscustomobject][ordered]@{
    RootProcessId = $RootProcessId
    RootCreatedAtUtc = $rootCreatedAt.ToUniversalTime().ToString('o')
    StartedAtUtc = $startedAt.ToString('o')
    EndedAtUtc = $endedAt.ToString('o')
    RequestedSampleSeconds = $SampleSeconds
    ActualSampleSeconds = [Math]::Round($timer.Elapsed.TotalSeconds, 6)
    SampleCount = $sampleCount
    LogicalProcessorCount = $logicalProcessors
    ProcessCount = $current.Count
    WebView2ProcessCount = @($current | Where-Object Name -IEQ 'msedgewebview2.exe').Count
    ObservedProcessCount = $observed.Count
    WorkingSetMiB = [Math]::Round([double]$workingSetBytes / 1MB, 3)
    PrivateBytesMiB = [Math]::Round([double]$privateBytes / 1MB, 3)
    CpuTimeSeconds = [Math]::Round($cpuSeconds, 6)
    CpuPercentMachine = [Math]::Round(100 * $cpuSeconds / $timer.Elapsed.TotalSeconds / $logicalProcessors, 4)
    Scope = 'One wordglance.exe plus msedgewebview2.exe reached through its live parent-child tree; PID and creation time identify each process.'
    MemoryMeasurement = 'End-of-sample snapshot. Working sets include shared pages, so summing can count shared pages more than once. PrivateBytes is private committed memory, not resident RAM.'
    CpuMeasurement = 'Observed CPU deltas at approximately 1-second intervals; CPU percent is normalized across all logical processors. Exited processes can lose their final interval; processes born and exited between polls are not observed.'
    Processes = $processDetails
}

if ($OutputJson) {
    $outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputJson)
    $parentDirectory = Split-Path -Parent $outputPath
    if (-not (Test-Path -LiteralPath $parentDirectory)) {
        New-Item -ItemType Directory -Path $parentDirectory -Force | Out-Null
    }
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $outputPath -Encoding UTF8
}
$result

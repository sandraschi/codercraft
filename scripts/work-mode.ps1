<#
.SYNOPSIS
    CoderCraft work-mode: free VRAM for local model inference, restore after.
.DESCRIPTION
    Sandra's box runs ~19GB of VRAM in desktop + idle apps, leaving ~3GB for
    Ollama. Every serious loop-brain model then spills to CPU and crawls
    (measured 60-145s per brain call). This script snapshots the killable
    GPU holders, stops them, and relaunches them afterwards.

    Actions:
      status    Show VRAM, Ollama residents, and current candidates. No changes.
      snapshot  Write candidate snapshot to ~/.craft/work-mode-snapshot.json.
      free      Snapshot, then stop candidates. DRY-RUN unless -Force is given.
      restore   Relaunch everything recorded by the last free.

    Default candidates (conservative):
      - D:\Dev\repos\*\target\debug\*.exe dev builds (wgpu holders),
        EXCEPT codercraft* (never kill our own fleet app mid-session)
      - LM Studio UI (holds no models right now, but reserves GPU context)
    With -IncludeCreative: also Blender and Mayo.
    Browsers, Discord, Docker, editors, security, and system processes are
    NEVER touched unless explicitly named in -ExtraProcesses.
.EXAMPLE
    .\work-mode.ps1 status
    .\work-mode.ps1 free
    .\work-mode.ps1 free -Force
    .\work-mode.ps1 free -Force -IncludeCreative
    .\work-mode.ps1 restore
#>
[CmdletBinding()]
param(
    [ValidateSet('status', 'snapshot', 'free', 'restore')]
    [string]$Action = 'status',
    [switch]$Force,
    [switch]$IncludeCreative,
    [string[]]$ExtraProcesses = @(),
    [string[]]$Keep = @()
)

$ErrorActionPreference = 'Continue'
$SnapshotPath = Join-Path $env:USERPROFILE '.craft/work-mode-snapshot.json'

function Get-Vram {
    $out = nvidia-smi --query-gpu=memory.used,memory.free --format=csv,noheader 2>$null
    if ($out -match '(\d+)\s*MiB,\s*(\d+)\s*MiB') {
        return @{ UsedMiB = [int]$Matches[1]; FreeMiB = [int]$Matches[2] }
    }
    return @{ UsedMiB = -1; FreeMiB = -1 }
}

function Get-OllamaResidents {
    try {
        $resp = Invoke-RestMethod -Uri 'http://127.0.0.1:11434/api/ps' -TimeoutSec 10
        return @($resp.models | ForEach-Object { $_.name })
    } catch {
        return @()
    }
}

function Get-Candidates {
    param([string[]]$Extra, [string[]]$KeepList, [bool]$Creative)
    $cands = @()
    $procs = Get-Process -ErrorAction SilentlyContinue
    foreach ($p in $procs) {
        $name = $p.ProcessName
        if ($KeepList -contains $name) { continue }
        $path = $null
        try { $path = $p.Path } catch { $path = $null }
        $reason = $null
        if ($path -like 'D:\Dev\repos\*\target\debug\*.exe') {
            if ($name -like 'codercraft*') { continue }
            $reason = 'dev-build GPU app (idle wgpu holder)'
        } elseif ($name -in @('LM Studio', 'lms')) {
            $reason = 'LM Studio shell (GPU context, no models loaded)'
        } elseif ($Creative -and ($name -in @('blender', 'mayo'))) {
            $reason = 'creative app (only with -IncludeCreative)'
        } elseif ($Extra -contains $name) {
            $reason = 'explicitly requested via -ExtraProcesses'
        }
        if ($reason) {
            $cands += [pscustomobject]@{
                Pid    = $p.Id
                Name   = $name
                Path   = $path
                Reason = $reason
            }
        }
    }
    return $cands
}

function Show-Status {
    $vram = Get-Vram
    Write-Host ("VRAM used={0} MiB free={1} MiB" -f $vram.UsedMiB, $vram.FreeMiB)
    $residents = Get-OllamaResidents
    Write-Host ("Ollama residents: " + ($(if ($residents.Count) { $residents -join ', ' } else { '(none)' })))
    $cands = Get-Candidates -Extra $ExtraProcesses -KeepList $Keep -Creative $IncludeCreative
    Write-Host ("Kill candidates ({0}):" -f $cands.Count)
    foreach ($c in $cands) {
        Write-Host ("  pid={0} {1} [{2}] path={3}" -f $c.Pid, $c.Name, $c.Reason, $c.Path)
    }
}

switch ($Action) {
    'status' {
        Show-Status
    }
    'snapshot' {
        $cands = Get-Candidates -Extra $ExtraProcesses -KeepList $Keep -Creative $IncludeCreative
        $snap = [pscustomobject]@{
            Timestamp = (Get-Date).ToString('o')
            Vram      = Get-Vram
            Ollama    = Get-OllamaResidents
            Candidates = $cands
            Killed    = @()
        }
        $snap | ConvertTo-Json -Depth 5 | Set-Content -Path $SnapshotPath -Encoding utf8
        Write-Host ("Snapshot written: {0} ({1} candidates)" -f $SnapshotPath, $cands.Count)
    }
    'free' {
        $before = Get-Vram
        $cands = Get-Candidates -Extra $ExtraProcesses -KeepList $Keep -Creative $IncludeCreative
        Write-Host ("VRAM before: used={0} MiB free={1} MiB" -f $before.UsedMiB, $before.FreeMiB)
        Write-Host ("Candidates ({0}):" -f $cands.Count)
        foreach ($c in $cands) {
            Write-Host ("  pid={0} {1} [{2}]" -f $c.Pid, $c.Name, $c.Reason)
        }
        if (-not $Force) {
            Write-Host 'DRY-RUN: nothing stopped. Re-run with -Force to stop the above.'
            return
        }
        $killed = @()
        foreach ($c in $cands) {
            $proc = Get-Process -Id $c.Pid -ErrorAction SilentlyContinue
            if (-not $proc) {
                Write-Host ("  already gone: pid={0} {1}" -f $c.Pid, $c.Name)
                continue
            }
            $closed = $false
            try { $closed = $proc.CloseMainWindow() } catch { $closed = $false }
            if ($closed) {
                $exited = $proc.WaitForExit(3000)
                if ($exited) {
                    Write-Host ("  closed gracefully: {0}" -f $c.Name)
                    $killed += $c
                    continue
                }
            }
            try {
                Stop-Process -Id $c.Pid -Force -ErrorAction Stop
                Write-Host ("  killed: {0}" -f $c.Name)
                $killed += $c
            } catch {
                Write-Host ("  FAILED to stop {0}: {1}" -f $c.Name, $_)
            }
        }
        Start-Sleep -Seconds 3
        $after = Get-Vram
        $snap = [pscustomobject]@{
            Timestamp = (Get-Date).ToString('o')
            Vram      = $before
            Ollama    = Get-OllamaResidents
            Candidates = $cands
            Killed    = $killed
        }
        $snap | ConvertTo-Json -Depth 5 | Set-Content -Path $SnapshotPath -Encoding utf8
        Write-Host ("VRAM after: used={0} MiB free={1} MiB (freed ~{2} MiB)" -f $after.UsedMiB, $after.FreeMiB, ($before.FreeMiB - $after.FreeMiB) * -1)
        Write-Host ("Snapshot: {0}. Use 'restore' to relaunch." -f $SnapshotPath)
    }
    'restore' {
        if (-not (Test-Path -LiteralPath $SnapshotPath)) {
            Write-Host ("No snapshot at {0}; nothing to restore." -f $SnapshotPath)
            return
        }
        $snap = Get-Content -LiteralPath $SnapshotPath -Raw | ConvertFrom-Json
        $ok = 0
        $failed = 0
        foreach ($k in $snap.Killed) {
            if (-not $k.Path) {
                Write-Host ("  no recorded path, skipping: {0}" -f $k.Name)
                $failed++
                continue
            }
            if (-not (Test-Path -LiteralPath $k.Path)) {
                Write-Host ("  path gone, skipping: {0}" -f $k.Path)
                $failed++
                continue
            }
            $already = Get-Process -Name $k.Name -ErrorAction SilentlyContinue
            if ($already) {
                Write-Host ("  already running: {0}" -f $k.Name)
                $ok++
                continue
            }
            try {
                Start-Process -FilePath $k.Path -WorkingDirectory (Split-Path -Parent $k.Path) -ErrorAction Stop
                Write-Host ("  relaunched: {0}" -f $k.Name)
                $ok++
            } catch {
                Write-Host ("  FAILED to relaunch {0}: {1}" -f $k.Name, $_)
                $failed++
            }
        }
        Write-Host ("Restore done: {0} ok, {1} failed/skipped." -f $ok, $failed)
    }
}

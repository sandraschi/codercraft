# CoderCraft start: launches native desktop app (egui)
param([switch]$Build = $false)
$ErrorActionPreference = "Stop"

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: cargo not found." -ForegroundColor Red
    exit 1
}

Set-Location -LiteralPath $PSScriptRoot
if ($Build) { & cargo build -p codercraft-app }
& cargo run -p codercraft-app

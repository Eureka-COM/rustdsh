[CmdletBinding()]
param([string]$BinDirectory = (Join-Path $env:USERPROFILE '.local\bin'))
$ErrorActionPreference = 'Stop'
$taskNode = (Get-Command node.exe -ErrorAction Stop).Source
$taskCli = Join-Path $PSScriptRoot 'cli.mjs'
Push-Location $PSScriptRoot
try {
    & npm.cmd ci --omit=dev
    if ($LASTEXITCODE -ne 0) { throw 'Dashboard dependency installation failed' }
} finally { Pop-Location }
New-Item -ItemType Directory -Force -Path $BinDirectory | Out-Null
$taskWrapper = "& '" + $taskNode.Replace("'", "''") + "' '" + $taskCli.Replace("'", "''") + "' @args`nexit `$LASTEXITCODE`n"
Set-Content -LiteralPath (Join-Path $BinDirectory 'rdsh-dashboard.ps1') -Value $taskWrapper -Encoding utf8NoBOM
Set-Content -LiteralPath (Join-Path $BinDirectory 'rdsh-dashboard.cmd') -Value '@echo off', 'pwsh.exe -NoLogo -NoProfile -File "%~dp0rdsh-dashboard.ps1" %*' -Encoding ascii
Write-Host "Installed rdsh-dashboard in $BinDirectory. Add that directory to PATH if needed."

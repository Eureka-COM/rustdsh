$taskNode = (Get-Command node.exe -ErrorAction Stop).Source
& $taskNode (Join-Path $PSScriptRoot 'cli.mjs') @args
exit $LASTEXITCODE

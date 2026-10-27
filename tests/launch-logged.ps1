$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('tower launch test '+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$entry = Join-Path $root "fake 'game'.ps1"
@'
param($Mode,$GameDir,$ErrorFile)
Write-Output 'stdout-marker'
[Console]::Error.WriteLine('ERROR stderr-marker')
exit 17
'@ | Set-Content -LiteralPath $entry -Encoding utf8
$wrapper = Join-Path $PSScriptRoot '../distribution/Launch-Logged.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $wrapper -Entry $entry -GameDir 'F:\Game With Spaces' -ErrorFile "$root/error.txt" -LogDirectory "$root/logs"
if ($LASTEXITCODE -ne 17) {throw "Wrong exit code: $LASTEXITCODE; $(Get-Content $root/error.txt -ErrorAction SilentlyContinue)"}
if ((Get-Content "$root/logs/stdout.log" -Raw) -notmatch 'stdout-marker') {throw 'Missing stdout'}
if ((Get-Content "$root/logs/stderr.log" -Raw) -notmatch 'stderr-marker') {throw 'Missing stderr'}
Write-Output 'PASS: hidden worker captures stdout, stderr and exit code with quoted paths.'
@'
param($Mode,$GameDir,$ErrorFile)
[IO.File]::WriteAllText($ErrorFile,'missing dependency')
throw 'test startup failure'
'@ | Set-Content -LiteralPath $entry -Encoding utf8
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $wrapper -Entry $entry -GameDir 'F:\Game With Spaces' -ErrorFile "$root/error.txt" -LogDirectory "$root/failure"
if ($LASTEXITCODE -eq 0) {throw 'Script exception incorrectly reported success'}
if ((Get-Content "$root/error.txt" -Raw) -ne 'missing dependency') {throw 'Missing script error summary'}
Write-Output 'PASS: script exceptions report failure and preserve the summary.'

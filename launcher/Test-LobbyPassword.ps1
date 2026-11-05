$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '../Launch-Seamless-Tower.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw $errors[0] }
$fn = $ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Assert-TowerLobbyPassword'}, $true)
. ([scriptblock]::Create($fn.Extent.Text))
$hash = [Security.Cryptography.SHA256]::Create()
try { $digest = [BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes('TestPassword123'))).Replace('-', '').ToLowerInvariant() }
finally { $hash.Dispose() }
$checks = 0
foreach ($locked in @($false, $true)) {
    $room = @{locked=$locked;password=$(if ($locked) {''} else {'TestPassword123'});password_sha256=$digest}
    $json = $room | ConvertTo-Json -Compress
    Assert-TowerLobbyPassword $json "[PASSWORD]`r`ncooppassword = TestPassword123 ; comment"
    $checks++
    foreach ($ini in @('cooppassword = OtherPassword', "cooppassword = TestPassword123`ncooppassword = OtherPassword", '[PASSWORD]', 'cooppassword = testpassword123')) {
        $failed = $false
        try { Assert-TowerLobbyPassword $json $ini } catch { $failed = $true }
        if (!$failed) { throw 'Invalid configuration was accepted.' }
        $checks++
    }
    $room.Remove('password_sha256')
    $failed = $false
    try { Assert-TowerLobbyPassword ($room | ConvertTo-Json -Compress) 'cooppassword = TestPassword123' } catch { $failed = $true }
    if (!$failed) { throw 'Stale room was accepted.' }
    $checks++
}
Write-Output "Lobby password checks passed: $checks"

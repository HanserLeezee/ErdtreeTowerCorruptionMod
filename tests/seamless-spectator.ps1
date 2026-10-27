$ErrorActionPreference = 'Stop'
$path = Join-Path (Split-Path $PSScriptRoot -Parent) 'Launch-Seamless-Tower.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
if ($errors.Count) {throw 'Launcher parse failed'}
$fn = $ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Set-TowerSpectatorMode'},$true)
if (!$fn) {throw 'Missing spectator configuration function'}
. ([scriptblock]::Create($fn.Extent.Text))
foreach ($newline in @("`n","`r`n")) {
    foreach ($setting in @('always_spectate_on_death = 0','always_spectate_on_death = 1'," `talways_spectate_on_death = 0",'')) {
        $inputText = @('[GAMEPLAY]',$setting,'[NETWORK]','cooppassword = test-only','save_file_extension = ertowercoop') -join $newline
        $result = Set-TowerSpectatorMode $inputText
        if ($result -notmatch '(?m)^always_spectate_on_death = 1\r?$') {throw 'Spectator mode not enabled'}
        if ($result -notmatch 'cooppassword = test-only' -or $result -notmatch 'save_file_extension = ertowercoop') {throw 'Unrelated configuration changed'}
        if ($result.IndexOf('always_spectate_on_death') -gt $result.IndexOf('[NETWORK]')) {throw 'Wrong INI section'}
        if ((Set-TowerSpectatorMode $result) -cne $result) {throw 'Not idempotent'}
    }
}
$source = [IO.File]::ReadAllText($path)
if ($source -notmatch '\$modified = Set-TowerSpectatorMode \$modified') {throw 'Launcher does not apply spectator mode'}
Write-Output '8 spectator configuration cases passed.'

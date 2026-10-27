$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$tokens=$null; $errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $root 'Launch-Seamless-Tower.ps1'),[ref]$tokens,[ref]$errors)
if ($errors.Count) {throw 'Launcher syntax invalid'}
$fn=$ast.Find({param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Set-TowerMultiplayerPreset'},$true)
. ([scriptblock]::Create($fn.Extent.Text))
$preset=[IO.File]::ReadAllText((Join-Path $root 'seamless-preset.ini'))
foreach ($newline in @("`n","`r`n")) {
    foreach ($scaling in @('',('[SCALING]'+$newline+'enemy_health_scaling = 100'+$newline+'boss_damage_scaling = 50'+$newline))) {
        $inputText='[GAMEPLAY]'+$newline+'allow_invaders = 1'+$newline+'allow_summons = 0'+$newline+$scaling+'[NETWORK]'+$newline+'cooppassword = test-only'+$newline+'[SAVE]'+$newline+'save_file_extension = custom'+$newline
        $result=Set-TowerMultiplayerPreset $inputText $preset
        foreach ($line in ($preset -split '\r?\n' | Where-Object {$_ -match '^\w+ = \d+$'})) {
            if (!$result.Contains($line)) {throw "Missing preset setting: $line"}
        }
        if (!$result.Contains('cooppassword = test-only') -or !$result.Contains('save_file_extension = custom')) {throw 'Private settings changed'}
        if ((Set-TowerMultiplayerPreset $result $preset) -cne $result) {throw 'Preset not idempotent'}
    }
}
foreach ($bad in @($preset.Replace('enemy_health_scaling = 100','enemy_health_scaling = -1'),$preset.Replace('allow_summons = 1','allow_summons = 3'),$preset.Replace('boss_damage_scaling = 20','cooppassword = 123'),($preset+"`nboss_damage_scaling = 20"))) {
    $rejected=$false
    try {Set-TowerMultiplayerPreset $inputText $bad | Out-Null} catch {$rejected=$true}
    if (!$rejected) {throw 'Invalid preset accepted'}
}
Write-Output 'PASS: 4 configuration layouts and 4 invalid presets; private values preserved.'

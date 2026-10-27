$ErrorActionPreference = 'Stop'
# Get-TowerPluginSections in both launch scripts: plugins/natives.toml (written by the launcher) -> ME3 [[natives]] sections
$root = Split-Path $PSScriptRoot -Parent
$package = Join-Path ([IO.Path]::GetTempPath()) ('tower-plugins-' + [guid]::NewGuid())
try {
    $plugins = Join-Path $package 'plugins'
    foreach ($name in @('fps', 'overlay')) {
        $dir = Join-Path $plugins $name
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        [IO.File]::WriteAllText((Join-Path $dir "$name.dll"), 'MZ')
    }
    $outside = Join-Path $package 'elsewhere.dll'
    [IO.File]::WriteAllText($outside, 'MZ')
    $natives = @"
# 额外 DLL 插件
[[natives]]
path = 'plugins/fps/fps.dll'
enabled = true

[[natives]]
path = 'plugins/overlay/overlay.dll'
enabled = false

[[natives]]
path = 'plugins/missing/gone.dll'

[[natives]]
path = '$($outside.Replace('\', '/'))'
load_after = [{ id = "ersc.dll", optional = true }]
"@
    [IO.File]::WriteAllText((Join-Path $plugins 'natives.toml'), $natives, (New-Object Text.UTF8Encoding($false)))
    foreach ($script in @('Launch-Preview.ps1', 'Launch-Seamless-Tower.ps1')) {
        $source = [IO.File]::ReadAllText((Join-Path $root $script))
        $block = [regex]::Match($source, '(?s)# Extra plugins \(plugins/natives\.toml.*?# End extra plugins')
        if (-not $block.Success) { throw "Missing plugin support: $script" }
        . ([scriptblock]::Create($block.Value))
        $sections = @(Get-TowerPluginSections $package 3>$null)
        if ($sections.Count -ne 2) { throw "$script : expected the enabled and the absolute plugin, got $($sections.Count)" }
        $fps = ([IO.Path]::GetFullPath((Join-Path (Join-Path $plugins 'fps') 'fps.dll'))).Replace('\', '/')
        if ($sections[0] -notmatch [regex]::Escape("path = '$fps'")) { throw "$script : relative path not resolved: $($sections[0])" }
        if ($sections[0] -match 'enabled') { throw "$script : launcher switch leaked into the ME3 profile" }
        if ($sections[1] -notmatch 'load_after') { throw "$script : plugin options must be kept" }
        if (@($sections | Where-Object { $_ -match 'overlay|gone' }).Count -ne 0) { throw "$script : disabled or missing plugins must be skipped" }
        $none = @(Get-TowerPluginSections (Join-Path $package 'no-such-package') 3>$null)
        if ($none.Count -ne 0) { throw "$script : no plugin file means no sections" }
    }
    'tower-plugins: ok'
} finally {
    Remove-Item -LiteralPath $package -Recurse -Force -ErrorAction SilentlyContinue
}

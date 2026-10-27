"""Build the playable release: the stage folder, its ZIP, and the launcher's incremental update data.

Upload to a GitHub release tagged v<N> (marked latest; never delete older releases, their packs stay referenced):
  ErdtreeTower-v<N>.zip                 full package for new players
  release-v<N>/tower-release.json       the launcher reads this from releases/latest
  release-v<N>/tower-pack-v<N>.bin      only file contents that no earlier release has
and the folder mirror-v<N>/releases to the domestic mirror bucket (same layout as GitHub's download URLs): --upload does
that with ossutil and checks the line (distribution/upload_mirror.py, which can also be re-run on its own).

Usage: python distribution/package.py [--previous <tower-release.json path or URL>] [--first-release] [--notes <file>] [--upload]
By default the previous manifest is taken from the latest GitHub release so unchanged files keep pointing at older packs.
"""
from pathlib import Path
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import urllib.error
import urllib.request
import zipfile
import re
from native_build import build_native
from release_pack import build as build_release

LATEST_MANIFEST = 'https://github.com/tamikip/erdtree-tower/releases/latest/download/tower-release.json'
parser = argparse.ArgumentParser(description='Build the release folder, ZIP and launcher update pack')
parser.add_argument('--previous', help='previous tower-release.json (path or https URL); default: the latest GitHub release')
parser.add_argument('--first-release', action='store_true', help='no earlier launcher release exists: every file goes into this pack')
parser.add_argument('--notes', type=Path, help='release notes shown by the launcher')
parser.add_argument('--display-version', required=True, help='package version shown to players, e.g. 1.0.0; internal build number stays independent')
parser.add_argument('--build-number', type=int, help='explicit monotonically increasing release build')
parser.add_argument('--upload', action='store_true', help='upload the mirror folder with ossutil afterwards (see upload_mirror.py)')
args = parser.parse_args()
if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', args.display_version):
    parser.error('--display-version must be major.minor.patch')
if args.upload:
    import upload_mirror
    upload_tool = upload_mirror.find_ossutil()  # fail before the long build, not after it
    upload_mirror.target_from_url(upload_mirror.mirror_url())

project = Path(__file__).resolve().parents[1]
source = project / 'release-preview'
dist = project / 'dist'
profile = (source / 'preview.me3').read_text(encoding='utf-8-sig')
native = re.findall(r"path\s*=\s*['\"](erdtree_tower_preview_v\d+\.dll)['\"]", profile)
if len(native) != 1:
    raise SystemExit('Expected one versioned native DLL in preview.me3')
version = re.search(r'_v(\d+)\.dll$', native[0]).group(1)
if args.build_number is not None:
    if args.build_number <= 0: parser.error('--build-number must be positive')
    version = str(args.build_number)
    updated_native = f'erdtree_tower_preview_v{version}.dll'
    profile = profile.replace(native[0], updated_native)
    native[0] = updated_native
stage = dist / f'ErdtreeTower-v{version}'
if stage.exists():
    raise SystemExit('Staging directory already exists; preserve it or choose a new release directory.')
built_native = build_native(project)
# Launcher: exe and its config at the package root, every other build output under launcher/ (App.config probes there)
launcher_build = Path(tempfile.mkdtemp(prefix='tower-launcher-'))
subprocess.run(['dotnet', 'build', str(project / 'launcher' / 'Tower.Launcher'), '-c', 'Release', '-o', str(launcher_build)], check=True)
stage.mkdir(parents=True)

def copy_file(src, relative):
    target = stage / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, target)

def copy_tree(src, relative):
    for item in src.rglob('*'):
        if item.is_symlink() or item.stat().st_file_attributes & 0x400:
            raise RuntimeError(f'Reparse point rejected: {item}')
        if item.is_file():
            if item.suffix.lower() in ('.pdb', '.log', '.bak'):
                continue
            if relative == 'scene-assets' and (item.name.endswith('-build.json') or item.name.endswith('-audit.tsv') or item.suffix.lower() == '.png'):
                continue
            copy_file(item, Path(relative) / item.relative_to(src))

copy_file(built_native, native[0])
for name in ['LICENSE', 'ADDITIONAL-TERMS.md']:
    copy_file(source / name, name)
(stage / 'preview.me3').write_text(profile, encoding='utf-8')
# Ship launcher sources directly so a stale release-preview copy cannot leak into the package.
for name in ['Launch-Preview.ps1', 'Launch-Seamless-Tower.ps1', 'New-PartyConfig.ps1', 'seamless-preset.ini', 'THIRD_PARTY_NOTICES.md']:
    copy_file(project / name, name)
for name in ['scene-assets', 'licenses']:
    copy_tree(project / name, name)
copy_tree(source / 'loader/bin', 'loader/bin')
for name in ['LICENSE-MIT', 'LICENSE-APACHE']:
    copy_file(source / 'loader' / name, 'loader/' + name)
for name in ['Start-Tower.ps1', 'Launch-Logged.ps1', '安装教程.md', '额外DLL插件说明.txt']:
    copy_file(Path(__file__).parent / name, name)
copy_file(project / 'reference/联机战斗验证.md', '联机战斗验证.md')
for name in ['ErdtreeTowerLauncher.exe', 'ErdtreeTowerLauncher.exe.config']:
    copy_file(launcher_build / name, name)
for library in sorted(launcher_build.glob('*.dll')):
    if library.name != 'Microsoft.Web.WebView2.Wpf.dll':
        copy_file(library, 'launcher/' + library.name)
copy_file(launcher_build / 'runtimes/win-x64/native/WebView2Loader.dll', 'launcher/WebView2Loader.dll')
copy_tree(launcher_build / 'launcher/ui', 'launcher/ui')
copy_file(launcher_build / 'launcher/sources.json', 'launcher/sources.json')
mirror_url = json.loads((launcher_build / 'launcher/sources.json').read_text(encoding='utf-8')).get('mirror', '')
if not mirror_url:
    print('Warning: launcher/sources.json has no mirror, players will only update from GitHub')
shutil.rmtree(launcher_build)
for mode in ['Solo', 'Seamless']:
    (stage / f'Start-{mode}.cmd').write_text(
        '@echo off\nsetlocal\n'
        '"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" '
        f'-NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-Tower.ps1" -Mode {mode} %*\n'
        'set "tower_exit_code=%errorlevel%"\nif not "%tower_exit_code%"=="0" pause\n'
        'exit /b %tower_exit_code%\n', encoding='ascii')
(stage / '安装教程.txt').write_text((stage / '安装教程.md').read_text(encoding='utf-8'), encoding='utf-8-sig')
files = sorted(p for p in stage.rglob('*') if p.is_file())
for file in files:
    name = file.relative_to(stage).as_posix()
    if (file.suffix in ['.log', '.sl2', '.co2', '.ertowercoop', '.ertowerparty', '.party-award'] or
            file.suffix.lower() in ['.pdb', '.bak'] or
            file.name in ['host.json', 'guest.json', 'GamePath.txt', 'tower-options.ini', 'tower-debug.enable', 'ersc_settings.ini'] or
            (name.startswith('tower-') and file.suffix in ['.json', '.jsonl', '.class', '.fp', '.history']) or
            '/runtime/' in name or '/party-runtime/' in name):
        raise RuntimeError(f'Private runtime file rejected: {name}')
manifest = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(stage).as_posix()}\n' for p in files)
(stage / 'SHA256SUMS.txt').write_text(manifest, encoding='utf-8')


def previous_manifest():
    if args.first_release:
        return None
    source = args.previous or LATEST_MANIFEST
    if not source.startswith('https://'):
        return Path(source)
    target = dist / f'previous-for-v{version}-tower-release.json'
    try:
        with urllib.request.urlopen(source, timeout=60) as response:
            target.write_bytes(response.read())
    except urllib.error.HTTPError as error:
        if error.code == 404:
            raise SystemExit('The latest GitHub release has no tower-release.json: pass --first-release for the first launcher '
                             'release, or --previous <file>. Stage kept at ' + str(stage))
        raise
    return target


release_dir = dist / f'release-v{version}'
release = build_release(stage, version, release_dir, previous_manifest(), args.notes.read_text(encoding='utf-8') if args.notes else '', args.display_version)
# Mirror upload folder: the same paths as GitHub's release download URLs
mirror_dir = dist / f'mirror-v{version}' / 'releases'
for folder in [mirror_dir / 'download' / release['tag'], mirror_dir / 'latest' / 'download']:
    folder.mkdir(parents=True, exist_ok=True)
    shutil.copy2(release_dir / 'tower-release.json', folder / 'tower-release.json')
shutil.copy2(release_dir / release['pack']['asset'], mirror_dir / 'download' / release['tag'] / release['pack']['asset'])
# The ZIP carries the install record, so the launcher in a freshly extracted package knows its exact version and files
(stage / 'tower-install.json').write_text(json.dumps({'version': release['version'], 'displayVersion': release['displayVersion'], 'files': release['files']}, ensure_ascii=False),
                                          encoding='utf-8')
archive = dist / f'ErdtreeTower-v{version}.zip'
with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as output:
    for file in sorted(stage.rglob('*')):
        if file.is_file():
            output.write(file, Path(stage.name) / file.relative_to(stage))
with zipfile.ZipFile(archive) as output:
    assert output.testzip() is None
checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
(dist / f'ErdtreeTower-v{version}.zip.sha256').write_text(f'{checksum}  {archive.name}\n', encoding='ascii')
print(f'Packaged {len(files) + 2} files, {archive.stat().st_size / 1024**2:.1f} MiB: {archive}')
print(f'SHA256 {checksum}')
print(f'Upload to GitHub release {release["tag"]} (latest): {archive.name}, {release_dir / "tower-release.json"}, '
      f'{release_dir / release["pack"]["asset"]}')
if args.upload:
    if upload_mirror.main([str(mirror_dir.parent), '--ossutil', upload_tool]) != 0:
        raise SystemExit(f'Mirror upload check failed; fix it and re-run: python distribution/upload_mirror.py {mirror_dir.parent}')
elif mirror_url:
    print(f'Mirror: python distribution/upload_mirror.py {mirror_dir.parent}  (or upload {mirror_dir} by hand, '
          f'download/ first, then latest/, and run check_mirror.py {mirror_url} --expect {version})')

"""Build the integrated armor appearance engine; no separate player installation."""
from pathlib import Path
import hashlib
import io
import os
import shutil
import subprocess
import urllib.request
import zipfile

def extract_asset(api, target):
    request = urllib.request.Request('https://api.github.com/' + api,
                                     headers={'Accept': 'application/octet-stream', 'User-Agent': 'ErdtreeTowerBuild'})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read()
    except OSError:
        data = subprocess.check_output(['gh', 'api', api, '-H', 'Accept: application/octet-stream'])
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        root = target.resolve()
        for name in archive.namelist():
            if not (root / name).resolve().is_relative_to(root):
                raise RuntimeError('Unsafe dependency archive path')
        archive.extractall(target)
    (target / 'archive.sha256').write_text(hashlib.sha256(data).hexdigest(), encoding='ascii')

def build_transmog(project):
    cache = project.parents[1] / 'work/tower-transmog-build'
    cache.mkdir(parents=True, exist_ok=True)
    cmake = os.environ.get('TOWER_CMAKE') or shutil.which('cmake')
    if not cmake:
        cmake_root = cache / 'cmake'
        executable = cmake_root / 'cmake-4.4.5-windows-x86_64/bin/cmake.exe'
        if not executable.exists():
            extract_asset('repos/Kitware/CMake/releases/assets/682701242', cmake_root)
        cmake = str(executable)
    sdk = Path(os.environ.get('TOWER_STEAMWORKS_SDK', str(cache / 'steamworks')))
    if not (sdk / 'include/steam/steam_api.h').exists():
        extract_asset('repos/julianxhokaxhiu/SteamworksSDKCI/releases/assets/157344737', sdk)
    build = cache / 'build'
    subprocess.run([cmake, '-S', str(project / 'native/vendor/tower-transmog'), '-B', str(build),
                    '-A', 'x64', '-DCMAKE_POLICY_VERSION_MINIMUM=3.5',
                    '-DSTEAMWORKS_ROOT=' + str(sdk)], check=True)
    subprocess.run([cmake, '--build', str(build), '--config', 'Release', '--parallel', '6'], check=True)
    return build / 'Release/tower_transmog.dll'

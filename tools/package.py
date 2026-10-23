"""Create a release ZIP from an explicit set of distributable files."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import tomllib
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def package(build=None, output=None, system=None, arch=None):
    system = system or {'Windows':'windows','Darwin':'macos','Linux':'linux'}[platform.system()]
    arch = arch or {'AMD64':'x86_64','x86_64':'x86_64','arm64':'aarch64','aarch64':'aarch64'}[platform.machine()]
    version = tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
    build = Path(build or ROOT/'build/release')
    output = Path(output or ROOT/'dist')
    name = f'Ada3D-v{version}-{system}-{arch}'
    output.mkdir(parents=True, exist_ok=True)
    # A user may be running an extracted build from dist. Stage independently.
    with tempfile.TemporaryDirectory(prefix=f'.{name}-', dir=output) as folder:
        stage = Path(folder)
        binary = 'Ada3D.exe' if system == 'windows' else 'Ada3D'
        if system == 'macos':
            subprocess.run(['sh',str(ROOT/'tools/package-macos.sh')],cwd=ROOT,check=True)
            shutil.copytree(ROOT/'dist/Ada3D.app',stage/'Ada3D.app')
        else:
            shutil.copy2(build/binary,stage/binary)
        for name in ['LICENSE','COPYRIGHT','THIRD_PARTY_NOTICES.md']:
            shutil.copy2(ROOT/name,stage/name)
        shutil.copytree(ROOT/'licenses',stage/'licenses')
        repo = os.environ.get('ADA3D_UPDATE_REPOSITORY','R3FStudio/Ada3D')
        if repo:
            source_ref = os.environ.get('GITHUB_SHA', f'v{version}')
            (stage/'SOURCE.txt').write_text(f'Ada3D {version}\nCopyright (C) 2026 R3F Studio\nCorresponding source: https://github.com/{repo}/tree/{source_ref}\nLicense: GNU GPLv3; see LICENSE.\n',encoding='utf-8')
        if system == 'linux':
            share = stage/'share'
            for src,dst in [('assets/linux/Ada3D.desktop','applications/Ada3D.desktop'),('assets/brand/mark.svg','icons/hicolor/scalable/apps/Ada3D.svg'),('assets/brand/app-icon.png','icons/hicolor/256x256/apps/Ada3D.png')]:
                target=share/dst;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/src,target)
            shutil.copy2(ROOT/'tools/install-linux-package.sh',stage/'install.sh')
        archive = output/f'Ada3D-v{version}-{system}-{arch}.zip'
        with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as bundle:
            for source in sorted(stage.rglob('*')):
                if source.is_symlink(): raise ValueError(f'Symlink in package: {source}')
                if not source.is_file(): continue
                relative = source.relative_to(stage).as_posix()
                if source.suffix.lower() in {'.log','.pdb','.dmp','.tmp'} or '.verification' in source.parts:
                    raise ValueError(f'Internal file in package: {relative}')
                info=zipfile.ZipInfo(relative)
                info.create_system=3
                executable = relative == binary or relative == 'install.sh' or relative == 'Ada3D.app/Contents/MacOS/Ada3D'
                info.external_attr=(stat.S_IFREG | (0o755 if executable else 0o644)) << 16
                info.compress_type=zipfile.ZIP_DEFLATED
                bundle.writestr(info,source.read_bytes())
        digest=hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_suffix('.zip.sha256').write_text(f'{digest}  {archive.name}\n',encoding='ascii')
        print(archive)
        return archive

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build',type=Path)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    package(args.build,args.output)

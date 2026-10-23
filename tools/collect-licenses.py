"""Collect upstream dependency licenses from the locked Cargo sources."""
import json
from pathlib import Path
import shutil
import subprocess
ROOT=Path(__file__).resolve().parents[1]
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=ROOT))
records=[]
for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
    if package['source'] is None: continue
    source=Path(package['manifest_path']).parent
    target=ROOT/'licenses/dependencies'/f"{package['name']}-{package['version']}"
    target.mkdir(parents=True,exist_ok=True)
    candidates=[p for p in source.iterdir() if p.is_file() and p.name.lower().startswith(('license','licence','copying','notice','copyright','unlicense'))]
    if package.get('license_file'):
        declared=source/package['license_file']
        if declared.is_file() and declared not in candidates:candidates.append(declared)
    if package['name'] == 'wayland-protocols-plasma':
        candidates.append(source/'plasma-wayland-protocols/COPYING.LIB')
        (target/'COPYRIGHT.blur').write_text('KDE blur protocol: Copyright (C) 2015 Martin Gräßlin; Copyright (C) 2015 Marco Martin. SPDX-License-Identifier: LGPL-2.1-or-later. See COPYING.LIB.\n',encoding='utf-8')
    for path in candidates:shutil.copy2(path,target/path.name)
    files=sorted(p.name for p in target.iterdir() if p.is_file())
    if not files:
        # Some crates publish their notice inside README rather than LICENSE.
        readmes=[p for p in source.iterdir() if p.is_file() and p.name.lower().startswith('readme')]
        for path in readmes:shutil.copy2(path,target/path.name)
        files=sorted(p.name for p in target.iterdir() if p.is_file())
    if not files and package['name'] in ('winapi-i686-pc-windows-gnu','winapi-x86_64-pc-windows-gnu'):
        upstream=ROOT/'licenses/dependencies/winapi-0.3.9'
        for path in upstream.glob('LICENSE*'):shutil.copy2(path,target/path.name)
        files=sorted(p.name for p in target.iterdir() if p.is_file())
    if not files:raise RuntimeError(f"Missing upstream notice for {package['name']}")
    records.append({'name':package['name'],'version':package['version'],'license':package['license'],'repository':package.get('repository'),'files':files})
(ROOT/'licenses/dependencies/inventory.json').write_text(json.dumps(records,indent=2)+'\n',encoding='utf-8')
print(f'Collected notices for {len(records)} packages.')

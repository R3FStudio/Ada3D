"""Remove Cargo build output; keep release packages unless --dist is given."""
import argparse
from pathlib import Path
import shutil
ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dist',action='store_true',help='Also remove release packages')
parser.add_argument('--dry-run',action='store_true')
args=parser.parse_args()
for name in ['build','target']+(['dist'] if args.dist else []):
    path=ROOT/name
    if not path.exists(): continue
    if path.is_symlink() or path.resolve().parent!=ROOT.resolve():
        raise SystemExit(f'Refusing to remove a path outside the project: {path}')
    print(('Would remove: ' if args.dry_run else 'Removing: ')+str(path.resolve()))
    if not args.dry_run: shutil.rmtree(path)

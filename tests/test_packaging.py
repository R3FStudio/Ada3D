"""Release archives must exclude build output and preserve executable modes."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('ada_package',ROOT/'tools/package.py')
packaging=importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)
class PackagingTests(unittest.TestCase):
    def test_windows_package_uses_an_explicit_file_list_and_checksum(self):
        with tempfile.TemporaryDirectory() as folder:
            folder=Path(folder);build=folder/'build';build.mkdir()
            (build/'Ada3D.exe').write_bytes(b'fixture executable')
            (build/'build.log').write_text('private log')
            (build/'Ada3D.pdb').write_text('debug symbols')
            old=folder/'dist/Ada3D-v1.0.0-windows-x86_64'
            old.mkdir(parents=True)
            (old/'Ada3D.exe').write_bytes(b'installed executable')
            archive=packaging.package(build,folder/'dist','windows','x86_64')
            self.assertEqual((old/'Ada3D.exe').read_bytes(),b'installed executable')
            self.assertFalse(any(p.name.startswith('.Ada3D-') for p in (folder/'dist').iterdir()))
            with zipfile.ZipFile(archive) as bundle:
                names=bundle.namelist()
                self.assertIn('Ada3D.exe',names)
                self.assertIn('LICENSE',names)
                self.assertIn('COPYRIGHT',names)
                self.assertNotIn('build.log',names)
                self.assertNotIn('Ada3D.pdb',names)
                self.assertFalse(any('.verification' in name or 'target/' in name for name in names))
            digest=hashlib.sha256(archive.read_bytes()).hexdigest()
            self.assertEqual(archive.with_suffix('.zip.sha256').read_text().split()[0],digest)
    def test_linux_package_preserves_launch_permissions_and_launcher_icon(self):
        with tempfile.TemporaryDirectory() as folder:
            folder=Path(folder);build=folder/'build';build.mkdir()
            (build/'Ada3D').write_bytes(b'fixture executable')
            archive=packaging.package(build,folder/'dist','linux','aarch64')
            with zipfile.ZipFile(archive) as bundle:
                self.assertEqual((bundle.getinfo('Ada3D').external_attr>>16)&0o777,0o755)
                self.assertEqual((bundle.getinfo('install.sh').external_attr>>16)&0o777,0o755)
                self.assertIn('share/applications/Ada3D.desktop',bundle.namelist())
                self.assertIn('share/icons/hicolor/scalable/apps/Ada3D.svg',bundle.namelist())
if __name__=='__main__':unittest.main()

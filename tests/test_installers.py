"""Verify installer input validation and package metadata without running an installer."""
import hashlib
import os
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile
import tomllib
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("installers",ROOT/"tools/installers.py")
installers=importlib.util.module_from_spec(spec);spec.loader.exec_module(installers)
class InstallerTests(unittest.TestCase):
    def test_checksum_failure_never_invokes_an_installer_builder(self):
        version=tomllib.loads((ROOT/"Cargo.toml").read_text())["package"]["version"]
        with tempfile.TemporaryDirectory() as folder:
            archive=Path(folder)/f"Ada3D-v{version}-windows-x86_64.zip"
            archive.write_bytes(b"corrupt download")
            archive.with_name(archive.name+".sha256").write_text("0"*64)
            with patch.object(installers.platform,"system",return_value="Windows"),patch.object(installers.platform,"machine",return_value="AMD64"),patch.object(installers,"windows") as builder:
                with self.assertRaisesRegex(ValueError,"checksum"): installers.build(archive)
                builder.assert_not_called()
    def test_release_path_traversal_is_rejected_before_extracting(self):
        version=tomllib.loads((ROOT/"Cargo.toml").read_text())["package"]["version"]
        with tempfile.TemporaryDirectory() as folder:
            archive=Path(folder)/f"Ada3D-v{version}-windows-x86_64.zip"
            with zipfile.ZipFile(archive,"w") as bundle: bundle.writestr("../escape.exe",b"bad")
            installers.checksum(archive)
            with patch.object(installers.platform,"system",return_value="Windows"),patch.object(installers.platform,"machine",return_value="AMD64"),patch.object(installers,"windows") as builder:
                with self.assertRaisesRegex(ValueError,"Unsafe"): installers.build(archive)
                builder.assert_not_called()
            self.assertFalse((Path(folder)/"escape.exe").exists())
    def test_native_builders_receive_the_matching_package_with_executable_mode(self):
        version=tomllib.loads((ROOT/"Cargo.toml").read_text())["package"]["version"]
        with tempfile.TemporaryDirectory() as folder:
            archive=Path(folder)/f"Ada3D-v{version}-linux-aarch64.zip"
            with zipfile.ZipFile(archive,"w") as bundle:
                entry=zipfile.ZipInfo("Ada3D");entry.external_attr=0o100755<<16;bundle.writestr(entry,b"fixture")
            installers.checksum(archive)
            def build(payload,work,output,actual_version,arch):
                self.assertEqual((payload/"Ada3D").read_bytes(),b"fixture")
                if os.name != "nt": self.assertEqual((payload/"Ada3D").stat().st_mode&0o777,0o755)
                self.assertEqual(arch,"aarch64")
                self.assertEqual(version,actual_version)
                result=output/"fixture.deb";result.write_bytes(b"archive");return [result]
            with patch.object(installers.platform,"system",return_value="Linux"),patch.object(installers.platform,"machine",return_value="aarch64"),patch.object(installers,"linux",side_effect=build):
                paths=installers.build(archive)
            self.assertEqual(paths[0].with_name("fixture.deb.sha256").read_text().split()[0],hashlib.sha256(b"archive").hexdigest())
if __name__=="__main__": unittest.main()

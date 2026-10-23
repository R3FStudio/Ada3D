"""Build self-contained native installers from a verified release ZIP."""
import argparse
import hashlib
import html
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "https://github.com/R3FStudio/Ada3D"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def checksum(path):
    path.with_name(path.name + ".sha256").write_text(f"{digest(path)}  {path.name}\n", encoding="ascii")


def artwork(folder):
    from PIL import Image, ImageDraw, ImageFont, ImageOps
    def font(size):
        for name in ("C:/Windows/Fonts/segoeui.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/System/Library/Fonts/Helvetica.ttc"):
            if Path(name).is_file():
                return ImageFont.truetype(name, size)
        return ImageFont.load_default(size=size)
    welcome = Image.new("RGB", (360, 690))
    draw = ImageDraw.Draw(welcome)
    for y in range(690):
        t = y / 690
        draw.line((0, y, 360, y), fill=(int(28+13*t), int(35+5*t), int(58+24*t)))
    for x,y,r in [(280,110,150),(40,530,130)]:
        draw.ellipse((x-r,y-r,x+r,y+r),outline=(70,83,139),width=2)
    logo=Image.open(ROOT/"assets/brand/app-icon.png").convert("RGBA")
    logo.thumbnail((144,144))
    welcome.paste(logo, ((360-logo.width)//2,100), logo)
    draw.text((44,300),"Ada3D",font=font(48),fill="white")
    draw.text((44,374),"Draw. Shape. Imagine.",font=font(22),fill=(192,205,241))
    draw.text((44,620),"R3F Studio",font=font(18),fill=(157,173,212))
    welcome.save(folder/"welcome.bmp")
    icon=Image.new("RGB",(64,64),(32,35,44)); logo.thumbnail((52,52));icon.paste(logo,((64-logo.width)//2,(64-logo.height)//2),logo);icon.save(folder/"logo.bmp")
    for source,target in [("shapes","drawing"),("scene-paint","scene")]:
        image=Image.open(ROOT/f"docs/images/{source}.png").convert("RGB")
        ImageOps.pad(image,(900,450),color=(29,32,40)).save(folder/f"{target}.bmp")


def windows(payload, work, output, version):
    images=work/"artwork";images.mkdir();artwork(images)
    compiler=os.environ.get("ISCC") or shutil.which("ISCC")
    if not compiler:
        for base in [os.environ.get("ProgramFiles(x86)",""),os.environ.get("ProgramFiles","")]:
            candidate=Path(base)/"Inno Setup 6/ISCC.exe"
            if candidate.is_file(): compiler=str(candidate);break
    if not compiler: raise RuntimeError("Install Inno Setup 6.7 or set ISCC to its compiler path.")
    subprocess.run([compiler,"/Q",f"/DPayload={payload}",f"/DOutput={output}",f"/DAppVersion={version}",f"/DArtwork={images}",f"/DSourceRoot={ROOT}",str(ROOT/"tools/windows-setup.iss")],check=True)
    return [output/f"Ada3D-v{version}-windows-x86_64-setup.exe"]


def linux(payload, work, output, version, arch):
    root=work/"root"
    binary=root/"usr/bin/Ada3D";binary.parent.mkdir(parents=True);shutil.copy2(payload/"Ada3D",binary);binary.chmod(0o755)
    shutil.copytree(payload/"share",root/"usr/share",dirs_exist_ok=True)
    notices=root/"usr/share/doc/ada3d";notices.mkdir(parents=True)
    for name in ["LICENSE","COPYRIGHT","THIRD_PARTY_NOTICES.md","SOURCE.txt","licenses"]:
        source=payload/name
        if source.is_dir(): shutil.copytree(source,notices/name)
        else: shutil.copy2(source,notices/name)
    meta=root/"usr/share/metainfo";meta.mkdir(parents=True)
    shutil.copy2(ROOT/"assets/linux/studio.r3f.Ada3D.metainfo.xml",meta)
    # Resolve the executable's actual linked libraries on the build runner.
    debian_work=work/"debian";debian_work.mkdir()
    (debian_work/"control").write_text("Source: ada3d\nSection: graphics\nPriority: optional\nMaintainer: R3F Studio\nStandards-Version: 4.6.0\n\nPackage: ada3d\nArchitecture: any\nDescription: Drawing and 3D scene editing\n")
    result=subprocess.run(["dpkg-shlibdeps","--ignore-missing-info","-O",f"-e{binary}"],cwd=work,text=True,capture_output=True,check=True)
    depends=next((line.split("=",1)[1] for line in result.stdout.splitlines() if line.startswith("shlibs:Depends=")),"libc6")
    debian=root/"DEBIAN";debian.mkdir()
    deb_arch={"x86_64":"amd64","aarch64":"arm64"}[arch]
    (debian/"control").write_text(f"Package: ada3d\nVersion: {version}\nArchitecture: {deb_arch}\nMaintainer: R3F Studio\nSection: graphics\nPriority: optional\nHomepage: {REPOSITORY}\nDepends: {depends}, libxkbcommon0\nRecommends: xdg-desktop-portal\nDescription: Drawing and 3D scene editing\n Draw, shape and paint in a native desktop workspace.\n",encoding="utf-8")
    deb=output/f"Ada3D-v{version}-linux-{arch}.deb"
    subprocess.run(["dpkg-deb","--root-owner-group","--build",str(root),str(deb)],check=True)
    shutil.rmtree(debian)
    rpmroot=work/"rpm";rpmroot.mkdir()
    spec=work/"ada3d.spec"
    spec.write_text(f"Name: ada3d\nVersion: {version}\nRelease: 1\nSummary: Drawing and 3D scene editing\nLicense: GPL-3.0-only\nURL: {REPOSITORY}\nBuildArch: {arch}\nRequires: libxkbcommon.so.0()(64bit)\n%description\nDraw, shape and paint in a native desktop workspace.\n%files\n/usr/bin/Ada3D\n/usr/share/applications/Ada3D.desktop\n/usr/share/icons/hicolor/scalable/apps/Ada3D.svg\n/usr/share/icons/hicolor/256x256/apps/Ada3D.png\n/usr/share/doc/ada3d\n/usr/share/metainfo/studio.r3f.Ada3D.metainfo.xml\n",encoding="utf-8")
    subprocess.run(["rpmbuild","-bb","--target",arch,"--buildroot",str(root),"--define",f"_topdir {rpmroot}","--define","_build_id_links none","--define","__os_install_post %{nil}",str(spec)],check=True)
    rpm=output/f"Ada3D-v{version}-linux-{arch}.rpm"
    built=list((rpmroot/"RPMS").rglob("*.rpm"))
    if len(built)!=1: raise RuntimeError("Expected one RPM installer")
    shutil.copy2(built[0],rpm)
    for command in [["dpkg-deb","--contents",str(deb)],["rpm","-qlp",str(rpm)]]:
        listing=subprocess.run(command,text=True,capture_output=True,check=True).stdout
        if "usr/bin/Ada3D" not in listing or any(name in listing for name in ["build.log",".pdb",".verification"]):
            raise RuntimeError("Native package content verification failed")
    return [deb,rpm]


def macos(payload,work,output,version,arch):
    root=work/"root";root.mkdir();shutil.copytree(payload/"Ada3D.app",root/"Ada3D.app")
    identity=os.environ.get("MACOS_APP_SIGN_IDENTITY")
    if identity:
        subprocess.run(["codesign","--force","--options","runtime","--timestamp","--sign",identity,str(root/"Ada3D.app")],check=True)
    component=work/"application.pkg"
    subprocess.run(["pkgbuild","--root",str(root),"--identifier","studio.r3f.Ada3D","--version",version,"--install-location","/Applications",str(component)],check=True)
    resources=work/"resources";resources.mkdir()
    (resources/"welcome.html").write_text('<html><body style="font-family: -apple-system, sans-serif"><h1>Ada3D</h1><p>Draw. Shape. Imagine.</p><img width="560" src="scene.png"><p>By R3F Studio</p></body></html>',encoding="utf-8")
    shutil.copy2(ROOT/"docs/images/scene-paint.png",resources/"scene.png")
    shutil.copy2(ROOT/"LICENSE",resources/"license.txt")
    distribution=work/"distribution.xml"
    distribution.write_text(f'''<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
<title>Ada3D</title><welcome file="welcome.html"/><license file="license.txt"/>
<options customize="never" require-scripts="false" hostArchitectures="{'arm64' if arch=='aarch64' else 'x86_64'}"/>
<domains enable_anywhere="false" enable_currentUserHome="false" enable_localSystem="true"/>
<choices-outline><line choice="ada3d"/></choices-outline>
<choice id="ada3d" visible="false"><pkg-ref id="studio.r3f.Ada3D"/></choice>
<pkg-ref id="studio.r3f.Ada3D" version="{html.escape(version)}">application.pkg</pkg-ref>
</installer-gui-script>''',encoding="utf-8")
    target=output/f"Ada3D-v{version}-macos-{arch}.pkg"
    command=["productbuild","--distribution",str(distribution),"--resources",str(resources),"--package-path",str(work)]
    if os.environ.get("MACOS_INSTALLER_SIGN_IDENTITY"): command += ["--sign",os.environ["MACOS_INSTALLER_SIGN_IDENTITY"]]
    subprocess.run(command+[str(target)],check=True)
    listing=subprocess.run(["pkgutil","--payload-files",str(component)],capture_output=True,text=True,check=True).stdout
    if "Ada3D.app/Contents/MacOS/Ada3D" not in listing: raise RuntimeError("App missing from installer")
    return [target]


def build(archive):
    archive=Path(archive).resolve()
    version=tomllib.loads((ROOT/"Cargo.toml").read_text())["package"]["version"]
    system={"Windows":"windows","Darwin":"macos","Linux":"linux"}[platform.system()]
    arch={"AMD64":"x86_64","x86_64":"x86_64","arm64":"aarch64","aarch64":"aarch64"}[platform.machine()]
    if archive.name!=f"Ada3D-v{version}-{system}-{arch}.zip": raise ValueError("Archive does not match this platform and version")
    expected=archive.with_name(archive.name+".sha256").read_text().split()[0]
    if digest(archive)!=expected: raise ValueError("Release ZIP checksum mismatch")
    with tempfile.TemporaryDirectory(prefix="ada3d-installers-") as directory:
        work=Path(directory);payload=work/"payload";payload.mkdir()
        with zipfile.ZipFile(archive) as bundle:
            for entry in bundle.infolist():
                if entry.filename.startswith("/") or ".." in Path(entry.filename).parts or "\\" in entry.filename: raise ValueError("Unsafe release path")
            bundle.extractall(payload)
            for entry in bundle.infolist():
                path=payload/entry.filename
                if path.is_file(): path.chmod((entry.external_attr>>16)&0o777 or 0o644)
        if system=="windows": paths=windows(payload,work,archive.parent,version)
        elif system=="macos": paths=macos(payload,work,archive.parent,version,arch)
        else: paths=linux(payload,work,archive.parent,version,arch)
        for path in paths: checksum(path); print(path)
    return paths

if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive",type=Path)
    build(parser.parse_args().archive)

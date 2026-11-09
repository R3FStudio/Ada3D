"""Check the real splash-to-workspace transition under a headless Wayland desktop."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def main():
    binary = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="ada3d-startup-") as directory:
        root = Path(directory)
        env = dict(os.environ, XDG_RUNTIME_DIR=directory, WAYLAND_DISPLAY="ada3d-test")
        env.pop("DISPLAY", None)
        env["LIBGL_ALWAYS_SOFTWARE"] = "1"
        with (root / "weston.log").open("w+") as log:
            compositor = subprocess.Popen(
                ["weston", "--backend=headless-backend.so", "--socket=ada3d-test",
                 "--idle-time=0", "--width=1600", "--height=1000"],
                env=env, stdout=log, stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 15
                while not (root / "ada3d-test").exists():
                    if compositor.poll() is not None or time.monotonic() >= deadline:
                        log.seek(0)
                        raise RuntimeError("Wayland desktop did not start:\n" + log.read())
                    time.sleep(0.1)
                for scale in [1.0, 1.5]:
                    capture = root / f"startup-{scale}.png"
                    subprocess.run(
                        [str(binary), "--startup", "--capture", str(capture),
                         "--ui-scale", str(scale)],
                        env=env, check=True, timeout=45,
                    )
                    result = json.loads(capture.with_suffix(".json").read_text())
                    assert capture.is_file() and result["success"], result
                    assert not result["splash"] and result["welcome_screen"], result
                    assert result["width"] >= 840 and result["height"] >= 560, result
                    print(f"Startup reached the workspace at UI scale {scale}.")
            finally:
                compositor.terminate()
                compositor.wait(timeout=10)


if __name__ == "__main__":
    main()

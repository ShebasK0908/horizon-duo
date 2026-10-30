#!/usr/bin/env python3
"""Build the Horizon WebAssembly browser client."""
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    cargo = Path(subprocess.check_output(["rustup", "which", "--toolchain", "stable", "cargo"], text=True).strip())
    rustc = cargo.with_name("rustc")
    bindgen = ROOT / ".local/web-tools/bin/wasm-bindgen"
    if not bindgen.exists():
        installed = shutil.which("wasm-bindgen")
        if not installed:
            raise SystemExit("Install wasm-bindgen-cli 0.2.129, matching Cargo.lock, into .local/web-tools.")
        bindgen = Path(installed)
    env = {**os.environ, "RUSTC": str(rustc), "PATH": str(cargo.parent) + os.pathsep + os.environ["PATH"]}
    subprocess.run([str(cargo), "build", "--locked", "-p", "horizon-web", "--target", "wasm32-unknown-unknown"], cwd=ROOT, env=env, check=True)
    subprocess.run([str(bindgen), "--target", "web", "--out-dir", str(ROOT / "web/pkg"),
                    str(ROOT / "target/wasm32-unknown-unknown/debug/horizon_web.wasm")], cwd=ROOT, check=True)

if __name__ == "__main__":
    main()

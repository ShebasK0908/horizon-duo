#!/usr/bin/env python3
"""Run a native Horizon server with disposable browser-test data and console input."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resources", type=Path, required=True)
    parser.add_argument("--port", type=int, default=25575)
    parser.add_argument("--resume", type=Path)
    args = parser.parse_args()
    local = ROOT / ".local"
    local.mkdir(exist_ok=True)
    if args.resume:
        directory = args.resume.resolve()
        if not directory.is_relative_to(local) or not (directory / ".horizon-web-test").is_file():
            raise SystemExit("Resume only a test directory created by this tool.")
    else:
        directory = Path(tempfile.mkdtemp(prefix="web-test-server-", dir=local))
        (directory / ".horizon-web-test").touch()
        (directory / "eula.txt").write_text("eula=true\n")
        (directory / "server.properties").write_text(
            f"server-ip=127.0.0.1\nserver-port={args.port}\nonline-mode=false\nwhite-list=false\nenforce-whitelist=false\n"
            "gamemode=creative\nlevel-name=web-proof\nlevel-type=minecraft:flat\nlevel-seed=1234\n"
            "generate-structures=false\nspawn-protection=0\nview-distance=4\nsimulation-distance=2\n"
            "max-players=4\nmax-tick-time=0\n")
    run = Path(tempfile.mkdtemp(prefix="run-", dir=directory))
    env = dict(os.environ, HORIZON_LAUNCHER_NO_SELF_UPDATE="1", HORIZON_NO_AUDIO="1")
    for variable, name in [("HORIZON_HOME", "home"), ("HORIZON_LAUNCHER_HOME", "launcher"),
                           ("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config")]:
        (run / name).mkdir()
        env[variable] = str(run / name)
    with (run / "server.log").open("w") as log:
        process = subprocess.Popen([str(ROOT / "target/debug/horizon-client"), "--resources",
            str(args.resources.resolve()), "--dedicated", "--nogui"], cwd=directory,
            env=env, stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT, text=True)
        print(f"Server {process.pid}. World directory {directory}. Log {run / 'server.log'}", flush=True)
        try:
            for command in sys.stdin:
                if process.poll() is not None: break
                process.stdin.write(command)
                process.stdin.flush()
        except KeyboardInterrupt:
            pass
        finally:
            if process.poll() is None:
                process.stdin.write("stop\n")
                process.stdin.flush()
                try: process.wait(timeout=30)
                except subprocess.TimeoutExpired: process.terminate()
        print(f"Server exit {process.poll()}", flush=True)

if __name__ == "__main__":
    main()

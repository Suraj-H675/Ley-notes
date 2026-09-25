#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import sys
import tempfile
from pathlib import Path


APP_IDENTIFIER = "app.leynotes.desktop"
EXPECTED_FILES = {
    "bindings-v1.json",
    "bindings-v1.lock",
    "projects-v1.json",
    "projects-v1.lock",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run the ordinary Ley CLI against an isolated OS-native config root and verify "
            "owner-only production registry permissions."
        )
    )
    parser.add_argument(
        "--require-all",
        action="store_true",
        help="fail instead of reporting unsupported on non-Linux/macOS hosts",
    )
    return parser.parse_args()


def ley_binary() -> Path:
    name = "ley.exe" if os.name == "nt" else "ley"
    return Path(__file__).resolve().parents[1] / "target" / "debug" / name


def run_ley(binary: Path, env: dict[str, str], args: list[str]) -> None:
    result = subprocess.run(
        [str(binary), *args],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"ley {' '.join(args)} failed: {detail}")


def unix_mode(path: Path) -> int:
    return stat.S_IMODE(path.lstat().st_mode)


def main() -> int:
    args = parse_args()
    if sys.platform not in {"linux", "darwin"}:
        payload = {
            "allPassed": not args.require_all,
            "supported": False,
            "system": sys.platform,
        }
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0 if payload["allPassed"] else 1

    binary = ley_binary()
    if not binary.is_file():
        raise RuntimeError(f"Ley CLI not found at {binary}; build the ordinary CLI first")

    with tempfile.TemporaryDirectory(prefix="ley-production-private-") as raw_base:
        base = Path(raw_base)
        home = base / "home"
        project = base / "project"
        vault = base / "vault"
        home.mkdir()
        project.mkdir()
        vault.mkdir()
        (project / "README.md").write_text("# Production private path probe\n", encoding="utf-8")

        env = os.environ.copy()
        env.pop("LEY_EVAL_PRIVATE_ROOT", None)
        env["HOME"] = str(home)
        env["USERPROFILE"] = str(home)
        if sys.platform == "linux":
            config_base = home / ".config"
            env["XDG_CONFIG_HOME"] = str(config_base)
        else:
            env.pop("XDG_CONFIG_HOME", None)
            config_base = home / "Library" / "Application Support"

        old_umask = os.umask(0)
        try:
            run_ley(binary, env, ["init", str(project), "--json"])
            run_ley(
                binary,
                env,
                ["bind", str(project), "--vault", str(vault), "--json"],
            )
        finally:
            os.umask(old_umask)

        application_dir = config_base / APP_IDENTIFIER
        if application_dir.is_symlink() or not application_dir.is_dir():
            raise RuntimeError("ordinary Ley config application directory was not created safely")
        directory_mode = unix_mode(application_dir)

        entries = {item.name for item in application_dir.iterdir()}
        missing = EXPECTED_FILES - entries
        if missing:
            raise RuntimeError(
                "ordinary Ley config is missing expected private files: "
                + ", ".join(sorted(missing))
            )

        file_modes: dict[str, str] = {}
        for name in sorted(EXPECTED_FILES):
            path = application_dir / name
            if path.is_symlink() or not path.is_file():
                raise RuntimeError(f"ordinary Ley private file is not a regular file: {name}")
            file_modes[name] = f"{unix_mode(path):04o}"

        all_passed = directory_mode == 0o700 and all(
            mode == "0600" for mode in file_modes.values()
        )
        payload = {
            "allPassed": all_passed,
            "applicationDirectoryMode": f"{directory_mode:04o}",
            "configConvention": "xdg" if sys.platform == "linux" else "macos-application-support",
            "fileModes": file_modes,
            "permissiveUmask": "0000",
            "supported": True,
            "system": "Linux" if sys.platform == "linux" else "Darwin",
        }
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0 if all_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.
"""
Vigo Browser — Release Build Pipeline

Orchestrates the full release build across all platforms:
  1. Version bumping
  2. Chromium/GN build
  3. Code signing
  4. Installer packaging (MSI, DMG/PKG, DEB/RPM)
  5. Update manifest generation
  6. Checksum computation
  7. Artifact upload (staging)

Usage:
    python build_release.py --version 0.1.0 --channel beta
    python build_release.py --version 1.0.0 --channel stable --sign

Environment Variables:
    VIGO_SIGN_CERT         Windows Authenticode certificate path (.pfx)
    VIGO_SIGN_PASSWORD     Certificate password
    APPLE_DEVELOPER_ID     Apple Developer ID (for codesign)
    APPLE_INSTALLER_ID     Apple Installer signing ID
    APPLE_ID               Apple ID for notarization
    APPLE_PASSWORD         App-specific password for notarization
    APPLE_TEAM_ID          Apple Developer Team ID
    GPG_KEY_ID             GPG key for Linux package signing
    VIGO_UPLOAD_URL        Staging server URL for artifact upload
    VIGO_UPLOAD_TOKEN      Authentication token for upload
"""

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


# ── Constants ────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent.resolve()
REPO_ROOT = SCRIPT_DIR.parent
VIGO_CORE = REPO_ROOT

CHANNELS = ("stable", "beta", "canary", "dev")
PLATFORMS = ("win", "mac", "linux")

UPDATE_BASE_URL = "https://update.vigobrowser.com"


# ── Helpers ──────────────────────────────────────────────────────

class Colors:
    CYAN = "\033[0;36m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[0;33m"
    RED = "\033[0;31m"
    RESET = "\033[0m"


def info(msg: str) -> None:
    print(f"{Colors.CYAN}[INFO]{Colors.RESET} {msg}")


def success(msg: str) -> None:
    print(f"{Colors.GREEN}[DONE]{Colors.RESET} {msg}")


def warn(msg: str) -> None:
    print(f"{Colors.YELLOW}[WARN]{Colors.RESET} {msg}")


def error(msg: str) -> None:
    print(f"{Colors.RED}[ERROR]{Colors.RESET} {msg}")
    sys.exit(1)


def run(cmd: list[str], cwd: str | None = None,
        check: bool = True) -> subprocess.CompletedProcess:
    """Run a subprocess with logging."""
    info(f"  $ {' '.join(cmd)}")
    return subprocess.run(
        cmd, cwd=cwd, check=check,
        capture_output=False, text=True
    )


def sha256_file(path: Path) -> str:
    """Compute SHA-256 of a file."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def detect_platform() -> str:
    """Detect current build platform."""
    system = platform.system().lower()
    if system == "windows":
        return "win"
    elif system == "darwin":
        return "mac"
    elif system == "linux":
        return "linux"
    else:
        error(f"Unsupported platform: {system}")
        return ""  # unreachable


# ── Version Management ───────────────────────────────────────────

def read_current_version() -> str:
    """Read version from vigo_branding.h."""
    branding = VIGO_CORE / "app" / "vigo_branding.h"
    if branding.exists():
        for line in branding.read_text().splitlines():
            if 'kVersionString[]' in line:
                # Extract "X.Y.Z" from the line.
                start = line.index('"') + 1
                end = line.index('"', start)
                return line[start:end]
    return "0.0.0"


def update_version(version: str) -> None:
    """Update version in vigo_branding.h."""
    branding = VIGO_CORE / "app" / "vigo_branding.h"
    if not branding.exists():
        warn("vigo_branding.h not found — skipping version update")
        return

    content = branding.read_text()
    old_version = read_current_version()
    new_content = content.replace(
        f'kVersionString[] = "{old_version}"',
        f'kVersionString[] = "{version}"'
    )

    # Also update major/minor/patch.
    parts = version.split(".")
    if len(parts) >= 3:
        major, minor, patch = parts[0], parts[1], parts[2]
        new_content = new_content.replace(
            f"kVersionMajor = {_extract_int(content, 'kVersionMajor')}",
            f"kVersionMajor = {major}"
        )
        new_content = new_content.replace(
            f"kVersionMinor = {_extract_int(content, 'kVersionMinor')}",
            f"kVersionMinor = {minor}"
        )
        new_content = new_content.replace(
            f"kVersionPatch = {_extract_int(content, 'kVersionPatch')}",
            f"kVersionPatch = {patch}"
        )

    branding.write_text(new_content)
    success(f"Version updated: {old_version} → {version}")


def _extract_int(content: str, name: str) -> str:
    """Extract an integer constant value from C++ source."""
    for line in content.splitlines():
        if name in line and "=" in line:
            return line.split("=")[1].strip().rstrip(";").strip()
    return "0"


# ── Build Steps ──────────────────────────────────────────────────

def step_gn_gen(build_dir: str, channel: str, is_official: bool) -> None:
    """Run GN gen with Vigo args."""
    info("Running GN gen...")

    args = [
        f'is_official_build={str(is_official).lower()}',
        'is_chrome_branded=false',
        'proprietary_codecs=true',
        'ffmpeg_branding="Chrome"',
        f'vigo_channel="{channel}"',
        'is_debug=false',
        'is_component_build=false',
        'symbol_level=1',
        'enable_nacl=false',
    ]

    plat = detect_platform()
    if plat == "win":
        args.append('target_os="win"')
    elif plat == "mac":
        args.append('target_os="mac"')
        args.append('target_cpu="x64"')  # TODO: universal binary
    elif plat == "linux":
        args.append('target_os="linux"')

    gn_args = " ".join(args)
    run(["gn", "gen", build_dir, f"--args={gn_args}"])
    success("GN gen complete")


def step_build(build_dir: str, jobs: int | None = None) -> None:
    """Run Ninja build."""
    info("Running Ninja build...")
    cmd = ["autoninja", "-C", build_dir, "vigo"]
    if jobs:
        cmd.extend(["-j", str(jobs)])
    run(cmd)
    success("Build complete")


def step_test(build_dir: str) -> None:
    """Run unit tests."""
    info("Running unit tests...")

    plat = detect_platform()
    ext = ".exe" if plat == "win" else ""

    test_binary = Path(build_dir) / f"vigo_unittests{ext}"
    if test_binary.exists():
        run([str(test_binary), "--gtest_brief=1"])
        success("Tests passed")
    else:
        warn(f"Test binary not found: {test_binary}")


def step_package_win(build_dir: str, sign: bool) -> list[Path]:
    """Build Windows installer."""
    info("Building Windows installer...")
    script = VIGO_CORE / "installer" / "win" / "build_installer.ps1"

    cmd = [
        "powershell", "-ExecutionPolicy", "Bypass", "-File", str(script),
        "-BuildDir", build_dir,
        "-OutputDir", str(VIGO_CORE / "installer" / "win" / "output"),
    ]

    if sign and os.environ.get("VIGO_SIGN_CERT"):
        cmd.extend(["-SignCert", os.environ["VIGO_SIGN_CERT"]])
        if os.environ.get("VIGO_SIGN_PASSWORD"):
            cmd.extend(["-SignPassword", os.environ["VIGO_SIGN_PASSWORD"]])
    else:
        cmd.append("-SkipSigning")

    run(cmd)

    output_dir = VIGO_CORE / "installer" / "win" / "output"
    return list(output_dir.glob("*.msi")) + list(output_dir.glob("*.sha256"))


def step_package_mac(build_dir: str, sign: bool) -> list[Path]:
    """Build macOS installer."""
    info("Building macOS installer...")
    script = VIGO_CORE / "installer" / "mac" / "create_installer.sh"

    cmd = ["bash", str(script), "--build-dir", build_dir]

    if sign:
        apple_id = os.environ.get("APPLE_DEVELOPER_ID", "")
        installer_id = os.environ.get("APPLE_INSTALLER_ID", "")
        if apple_id:
            cmd.extend(["--sign-id", apple_id])
        if installer_id:
            cmd.extend(["--installer-id", installer_id])
    else:
        cmd.append("--no-sign")

    run(cmd)

    output_dir = VIGO_CORE / "installer" / "mac" / "output"
    return list(output_dir.glob("*.dmg")) + list(output_dir.glob("*.pkg")) + \
           list(output_dir.glob("*.sha256"))


def step_package_linux(build_dir: str, sign: bool) -> list[Path]:
    """Build Linux packages."""
    info("Building Linux packages...")
    script = VIGO_CORE / "installer" / "linux" / "build_packages.sh"

    cmd = ["bash", str(script), "--build-dir", build_dir, "--all"]

    if sign and os.environ.get("GPG_KEY_ID"):
        cmd.extend(["--sign-key", os.environ["GPG_KEY_ID"]])

    run(cmd)

    output_dir = VIGO_CORE / "installer" / "linux" / "output"
    return list(output_dir.glob("*.deb")) + list(output_dir.glob("*.rpm")) + \
           list(output_dir.glob("*.sha256"))


# ── Update Manifest ──────────────────────────────────────────────

def generate_update_manifest(
    version: str, channel: str, artifacts: list[Path]
) -> Path:
    """Generate JSON update manifest for the auto-update system."""
    info("Generating update manifest...")

    manifest_entries = []

    for artifact in artifacts:
        if artifact.suffix in (".msi", ".dmg", ".pkg", ".deb", ".rpm"):
            # Determine platform from path.
            name = artifact.name
            if "win" in str(artifact):
                plat = "win"
            elif "mac" in str(artifact):
                plat = "mac"
            else:
                plat = "linux"

            entry = {
                "platform": plat,
                "filename": name,
                "version": version,
                "channel": channel,
                "url": f"{UPDATE_BASE_URL}/{channel}/{plat}/{name}",
                "sha256": sha256_file(artifact),
                "size": artifact.stat().st_size,
                "signature": "",  # Filled by signing step.
            }
            manifest_entries.append(entry)

    manifest = {
        "schema_version": 1,
        "product": "vigo-browser",
        "version": version,
        "channel": channel,
        "release_date": datetime.now(timezone.utc).isoformat(),
        "release_notes_url": f"https://vigobrowser.com/releases/{version}",
        "artifacts": manifest_entries,
    }

    output = VIGO_CORE / "installer" / f"update-manifest-{channel}.json"
    output.write_text(json.dumps(manifest, indent=2))
    success(f"Update manifest: {output}")
    return output


# ── Main Pipeline ────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Vigo Browser Release Build Pipeline"
    )
    parser.add_argument(
        "--version", required=True,
        help="Version string (e.g., 0.1.0, 1.0.0)"
    )
    parser.add_argument(
        "--channel", required=True, choices=CHANNELS,
        help="Release channel"
    )
    parser.add_argument(
        "--build-dir", default="out/Release",
        help="Build output directory"
    )
    parser.add_argument(
        "--sign", action="store_true",
        help="Enable code signing"
    )
    parser.add_argument(
        "--skip-build", action="store_true",
        help="Skip GN gen + Ninja build (use existing build)"
    )
    parser.add_argument(
        "--skip-test", action="store_true",
        help="Skip unit tests"
    )
    parser.add_argument(
        "--platform", choices=PLATFORMS,
        help="Build for specific platform only (default: auto-detect)"
    )
    parser.add_argument(
        "--jobs", type=int, default=None,
        help="Ninja parallel job count"
    )

    args = parser.parse_args()

    plat = args.platform or detect_platform()

    print()
    print("═══════════════════════════════════════════════════")
    print(" Vigo Browser — Release Build Pipeline")
    print("═══════════════════════════════════════════════════")
    print()
    info(f"Version:  {args.version}")
    info(f"Channel:  {args.channel}")
    info(f"Platform: {plat}")
    info(f"Signing:  {'yes' if args.sign else 'no'}")
    print()

    start_time = time.time()

    # Step 1: Version bump.
    info("[1/6] Updating version...")
    update_version(args.version)

    # Step 2: GN gen + build.
    if not args.skip_build:
        info("[2/6] Building...")
        is_official = args.channel in ("stable", "beta")
        step_gn_gen(args.build_dir, args.channel, is_official)
        step_build(args.build_dir, args.jobs)
    else:
        info("[2/6] Skipping build (--skip-build)")

    # Step 3: Test.
    if not args.skip_test:
        info("[3/6] Testing...")
        step_test(args.build_dir)
    else:
        info("[3/6] Skipping tests (--skip-test)")

    # Step 4: Package.
    info("[4/6] Packaging...")
    artifacts: list[Path] = []

    if plat == "win":
        artifacts = step_package_win(args.build_dir, args.sign)
    elif plat == "mac":
        artifacts = step_package_mac(args.build_dir, args.sign)
    elif plat == "linux":
        artifacts = step_package_linux(args.build_dir, args.sign)

    # Step 5: Update manifest.
    info("[5/6] Generating update manifest...")
    manifest = generate_update_manifest(args.version, args.channel, artifacts)
    artifacts.append(manifest)

    # Step 6: Summary.
    elapsed = time.time() - start_time
    minutes = int(elapsed // 60)
    seconds = int(elapsed % 60)

    print()
    print("═══════════════════════════════════════════════════")
    print(f" Release build complete! ({minutes}m {seconds}s)")
    print("═══════════════════════════════════════════════════")
    print()
    info(f"Version:   {args.version}")
    info(f"Channel:   {args.channel}")
    info(f"Platform:  {plat}")
    info(f"Artifacts: {len(artifacts)}")
    print()
    for a in artifacts:
        print(f"  {a.name}")
    print()

    # Upload hint.
    if os.environ.get("VIGO_UPLOAD_URL"):
        info("Upload artifacts with:")
        for a in artifacts:
            print(f"  curl -X PUT -H 'Authorization: Bearer $VIGO_UPLOAD_TOKEN' "
                  f"-F 'file=@{a}' "
                  f"$VIGO_UPLOAD_URL/{args.channel}/{plat}/")
    else:
        info("Set VIGO_UPLOAD_URL to enable automatic artifact upload.")


if __name__ == "__main__":
    main()

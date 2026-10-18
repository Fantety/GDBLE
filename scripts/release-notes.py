"""Validate the versioned UTF-8 Markdown used as a GitHub release body."""

import argparse
import re
import sys
import tomllib
from pathlib import Path


def release_notes_path(root: Path, tag: str) -> Path:
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", tag):
        raise ValueError("Expected a version tag such as v0.6.0 or v0.7.0-rc.1")
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    if tag[1:] != manifest["package"]["version"]:
        raise ValueError(f"Tag {tag} does not match Cargo.toml version")
    path = root / "release-notes" / f"{tag}.md"
    if not path.read_text(encoding="utf-8").strip():
        raise ValueError(f"Release notes are empty: {path}")
    return path


def write_github_outputs(root: Path, tag: str, output: Path) -> Path:
    path = release_notes_path(root, tag)
    # A hyphen in build metadata (e.g. +build-linux) is not a prerelease.
    prerelease = "-" in tag.partition("+")[0]
    with output.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(f"notes_path={path.relative_to(root).as_posix()}\n")
        stream.write(f"prerelease={str(prerelease).lower()}\n")
    return path


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()
    try:
        root = Path(__file__).resolve().parent.parent
        if args.github_output:
            print(write_github_outputs(root, args.tag, args.github_output))
        else:
            print(release_notes_path(root, args.tag))
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        print(f"Invalid release notes: {error}", file=sys.stderr)
        sys.exit(1)

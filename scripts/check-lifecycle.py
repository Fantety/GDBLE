"""Check the worker/main-thread boundary using only the Python standard library."""

import re
import sys
from pathlib import Path


def check_lifecycle(source_dir: Path) -> list[str]:
    sources = sorted(source_dir.rglob("*.rs"))
    if not sources:
        return [f"No Rust sources found in {source_dir}"]

    forbidden = (
        (r"block_on|unbounded_channel|mpsc::unbounded|tokio::spawn", "untracked or blocking async work"),
        (r"Arc<Mutex<[^>]*Gd<", "Godot handle shared across threads"),
    )
    errors = []
    for path in sources:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for pattern, reason in forbidden:
                if re.search(pattern, line):
                    errors.append(f"{path}:{line_number}: {reason}: {line.strip()}")

    manager = source_dir / "bluetooth_manager.rs"
    if not manager.is_file():
        errors.append(f"Missing {manager}")
    elif manager.read_text(encoding="utf-8").count("EVENTS_PER_FRAME: usize = 256") != 1:
        errors.append(f"{manager}: expected exactly one event budget of 256 per frame")
    return errors


if __name__ == "__main__":
    errors = check_lifecycle(Path(__file__).resolve().parent.parent / "src")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("Static lifecycle checks passed.")

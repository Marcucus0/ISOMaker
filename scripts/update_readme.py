#!/usr/bin/env python3
"""Refresh the generated project statistics section in README.md."""

from __future__ import annotations

import datetime as dt
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
START = "<!-- README:GENERATED:START -->"
END = "<!-- README:GENERATED:END -->"
BOT_SUBJECT = "chore(readme): refresh dynamic stats"


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    )
    return result.stdout.strip()


def count_commits() -> int:
    subjects = git("log", "--format=%s").splitlines()
    return sum(not subject.startswith(BOT_SUBJECT) for subject in subjects)


def latest_release() -> str:
    tags = git("tag", "--list", "v*", "--sort=-version:refname").splitlines()
    if tags:
        tag = tags[0]
        try:
            date = git("log", "-1", "--format=%cs", tag)
        except subprocess.CalledProcessError:
            date = "date inconnue"
        return f"`{tag}` · {date}"

    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    entry = re.search(r"^##\s+(\d+\.\d+\.\d+)\s+[—-]\s+(\d{4}-\d{2}-\d{2})", changelog, re.M)
    if entry:
        return f"`{entry.group(1)}` · {entry.group(2)} ([changelog](CHANGELOG.md); sans tag Git)"
    return "Aucune release documentée"


def source_stats() -> tuple[int, int, int, int]:
    paths = [path for path in git("ls-files").splitlines() if path]
    rust_paths = [p for p in paths if p.endswith(".rs")]
    python_paths = [p for p in paths if p.endswith(".py")]
    doc_paths = [p for p in paths if p.startswith("docs/") and p.endswith(".md")]

    def lines(files: list[str]) -> int:
        total = 0
        for relative in files:
            path = ROOT / relative
            try:
                total += len(path.read_text(encoding="utf-8").splitlines())
            except (OSError, UnicodeError):
                continue
        return total

    return len(rust_paths), lines(rust_paths), len(python_paths), len(doc_paths)


def generated_section() -> str:
    manifest = (ROOT / "crates/editor/Cargo.toml").read_text(encoding="utf-8")
    version_match = re.search(r'^version\s*=\s*"([^"]+)"', manifest, re.M)
    version = version_match.group(1) if version_match else "inconnue"
    try:
        commit = git("log", "-1", "--format=%h (%cs)")
    except subprocess.CalledProcessError:
        commit = "indisponible"
    rust_files, rust_lines, python_files, docs = source_stats()
    date = dt.datetime.now().astimezone().strftime("%Y-%m-%d")

    return "\n".join(
        [
            f"**Version actuelle :** `{version}`<br>",
            f"**Dernière release :** {latest_release()}<br>",
            f"**Commits du dépôt :** {count_commits()}<br>",
            f"**Dernier commit :** `{commit}`<br>",
            f"**Code et documentation :** {rust_files} fichiers Rust · {rust_lines:,} lignes Rust · {python_files} fichiers Python · {docs} documents Markdown"
            .replace(",", "\u202f"),
            f"\n_Mis à jour le {date}._",
        ]
    )


def main() -> int:
    content = README.read_text(encoding="utf-8")
    if START not in content or END not in content:
        print("README.md ne contient pas les marqueurs de génération attendus.", file=sys.stderr)
        return 1
    before, rest = content.split(START, 1)
    _, after = rest.split(END, 1)
    updated = f"{before}{START}\n{generated_section()}\n{END}{after}"
    README.write_text(updated, encoding="utf-8", newline="\n")
    print("README.md actualisé.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

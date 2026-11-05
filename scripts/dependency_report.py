#!/usr/bin/env python3
"""Write a Markdown report of direct dependency versions.

The report is advisory. It never fails the build. It uses cargo metadata for
resolved versions and the crates.io API for the latest stable release.
"""

from __future__ import annotations

import json
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request

CRATES_IO = "https://crates.io/api/v1/crates/"
USER_AGENT = "latchix-dependency-report (+https://github.com/Muyoouu/Latchix)"


def cargo_metadata() -> dict:
    output = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        text=True,
    )
    return json.loads(output)


def collect_direct_dependencies(metadata: dict) -> dict:
    packages = {package["id"]: package for package in metadata["packages"]}
    members = set(metadata["workspace_members"])
    dependencies: dict[str, dict] = {}

    for node in metadata["resolve"]["nodes"]:
        if node["id"] not in members:
            continue

        member = packages[node["id"]]["name"]
        for dependency in node["deps"]:
            package = packages[dependency["pkg"]]
            if package.get("source") is None:
                continue

            entry = dependencies.setdefault(
                package["name"],
                {"versions": set(), "members": set(), "kinds": set()},
            )
            entry["versions"].add(package["version"])
            entry["members"].add(member)
            for kind in dependency.get("dep_kinds", []):
                entry["kinds"].add(kind.get("kind") or "normal")

    return dependencies


def latest_stable(name: str) -> tuple[str | None, str | None]:
    url = CRATES_IO + urllib.parse.quote(name)
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            payload = json.load(response)
        crate = payload["crate"]
        return crate.get("max_stable_version") or crate.get("max_version"), None
    except (urllib.error.URLError, KeyError, TimeoutError) as error:
        return None, str(error)


def main() -> None:
    metadata = cargo_metadata()
    dependencies = collect_direct_dependencies(metadata)

    print("## Dependency freshness")
    print()
    print("Advisory report. Review each row. No pull request is opened.")
    print()
    print("| Crate | Resolved | Latest stable | Status | Kinds | Workspace members |")
    print("|---|---|---|---|---|---|")

    for name in sorted(dependencies):
        entry = dependencies[name]
        latest, error = latest_stable(name)
        resolved = ", ".join(sorted(entry["versions"]))
        kinds = ", ".join(sorted(entry["kinds"]))
        members = ", ".join(sorted(entry["members"]))
        if error is not None:
            latest_text = f"unknown ({error})"
            status = "unknown"
        elif latest is None:
            latest_text = "unknown"
            status = "unknown"
        else:
            latest_text = latest
            status = "review" if any(version != latest for version in entry["versions"]) else "ok"
        print(
            f"| {name} | {resolved} | {latest_text} | {status} | {kinds} | {members} |"
        )
        time.sleep(0.1)


if __name__ == "__main__":
    main()

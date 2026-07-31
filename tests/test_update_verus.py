# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the Verus update script."""

from __future__ import annotations

import json
import shlex
import subprocess
import unittest
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_UPDATE_SCRIPT = _REPO_ROOT / "scripts" / "update-verus.sh"


class TestUpdateVerus(unittest.TestCase):
    """Tests for Verus release and crate version resolution."""

    def run_update_script(self, body: str) -> str:
        """Sources the updater, runs a test body, and returns standard output."""
        command = f"source {shlex.quote(str(_UPDATE_SCRIPT))}\n{body}"
        result = subprocess.run(
            ["bash", "-c", command],
            cwd=_REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def test_resolve_latest_release_records_publication_time(self) -> None:
        """Stable release resolution records its version, commit, and publication time."""
        tags = [
            {"name": "release/rolling/0.2026.07.29.d8d931c"},
            {"name": "release/0.2026.07.27.31579f0"},
            {"name": "release/0.2026.07.18.3a4d30b"},
        ]
        release = {"published_at": "2026-07-27T02:45:29Z"}
        commit = {"sha": "31579f0b8542a8a9ae4ae5604c16107ccde23ef2"}
        tags_json = shlex.quote(json.dumps(tags))
        release_json = shlex.quote(json.dumps(release))
        commit_json = shlex.quote(json.dumps(commit))

        output = self.run_update_script(f"""
github_api_get() {{
    case "$1" in
        "repos/verus-lang/verus/tags?per_page=100")
            printf '%s\\n' {tags_json}
            ;;
        "repos/verus-lang/verus/releases/tags/release%2F0.2026.07.27.31579f0")
            printf '%s\\n' {release_json}
            ;;
        "repos/verus-lang/verus/commits/release/0.2026.07.27.31579f0")
            printf '%s\\n' {commit_json}
            ;;
        *)
            return 1
            ;;
    esac
}}
resolve_latest_release
printf '%s|%s|%s\\n' "$LATEST_VERSION" "$LATEST_COMMIT" "$LATEST_RELEASED_AT"
""")

        self.assertEqual(
            output,
            "0.2026.07.27.31579f0|"
            "31579f0b8542a8a9ae4ae5604c16107ccde23ef2|"
            "2026-07-27T02:45:29Z",
        )

    def test_resolve_vstd_version_uses_release_publication_time(self) -> None:
        """The resolver selects the newest non-yanked crate available at release time."""
        versions = {
            "versions": [
                {
                    "num": "0.0.0-2026-07-28-0100",
                    "created_at": "2026-07-28T01:00:00Z",
                    "yanked": False,
                },
                {
                    "num": "0.0.0-2026-07-27-0207",
                    "created_at": "2026-07-27T02:07:30Z",
                    "yanked": True,
                },
                {
                    "num": "0.0.0-2026-07-27-0206",
                    "created_at": "2026-07-27T02:07:22Z",
                    "yanked": False,
                },
                {
                    "num": "0.0.0-2026-07-12-0122",
                    "created_at": "2026-07-12T01:23:06Z",
                    "yanked": False,
                },
            ]
        }
        versions_json = shlex.quote(json.dumps(versions))

        output = self.run_update_script(f"""
crates_io_api_get() {{
    [[ "$1" == "crates/vstd/versions" ]]
    printf '%s\\n' {versions_json}
}}
LATEST_VERSION="0.2026.07.27.31579f0"
LATEST_RELEASED_AT="2026-07-27T02:45:29Z"
resolve_vstd_version
printf '%s\\n' "$NEW_VSTD"
""")

        self.assertEqual(output, "0.0.0-2026-07-27-0206")

    def test_crates_io_api_get_sets_user_agent(self) -> None:
        """crates.io requests identify the Nanvix updater."""
        output = self.run_update_script("""
curl() {
    printf '%s\\n' "$@"
}
crates_io_api_get "crates/vstd/versions"
""")

        arguments = output.splitlines()
        self.assertIn("--user-agent", arguments)
        self.assertIn(
            "nanvix-verus-updater (https://github.com/nanvix/nanvix)",
            arguments,
        )
        self.assertIn(
            "https://crates.io/api/v1/crates/vstd/versions",
            arguments,
        )


if __name__ == "__main__":
    unittest.main()

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Guards for .github/workflows/test-change-review.yml.

The workflow runs on `pull_request_target`, so it has a write token. These tests keep it
consistent with `scripts/flag-test-changes.py` and keep it from ever running pull request code.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO_ROOT / "scripts"))

from testlayout import flagging  # noqa: E402

_WORKFLOW = _REPO_ROOT / ".github" / "workflows" / "test-change-review.yml"


class TestChangeReviewWorkflow(unittest.TestCase):
    """Static checks of the workflow definition."""

    text: str

    @classmethod
    def setUpClass(cls) -> None:
        cls.text = _WORKFLOW.read_text(encoding="utf-8")

    def test_comment_marker_matches_the_script(self) -> None:
        self.assertIn(f'body-includes: "{flagging.MARKER}"', self.text)

    def test_label_is_declared(self) -> None:
        self.assertIn('REVIEW_LABEL: "review::tests-modified"', self.text)
        self.assertIn("review%3A%3Atests-modified", self.text)

    def test_pull_request_code_is_never_checked_out(self) -> None:
        self.assertIn("pull_request_target:", self.text)
        self.assertNotIn("pull_request.head", self.text)
        self.assertEqual(self.text.count("actions/checkout@"), 1)
        self.assertNotIn("ref:", self.text)

    def test_default_token_has_no_permissions(self) -> None:
        self.assertIn("\npermissions: {}\n", self.text)

    def test_scripts_never_interpolate_expressions(self) -> None:
        in_run = False
        for line in self.text.splitlines():
            stripped = line.strip()
            if stripped.startswith("run:"):
                in_run = True
                continue
            if in_run and stripped and not line.startswith(" " * 10):
                in_run = False
            if in_run:
                self.assertNotIn("${{", line)


if __name__ == "__main__":
    unittest.main()

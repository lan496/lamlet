import json
from pathlib import Path
import shutil
import tempfile
import unittest

from corpus import write_json
from repair_campaign import digest, ROOT
from repair_evidence import verify


ARCHIVE = ROOT / "docs/assets/evaluation/2026-11-09-repair"


class RepairEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.archive = Path(self.temporary.name) / "archive"
        shutil.copytree(ARCHIVE, self.archive)

    def rehash(self):
        write_json(self.archive / "hashes.json", {
            str(p.relative_to(self.archive)): digest(p)
            for p in sorted(self.archive.rglob("*"))
            if p.is_file() and p.name != "hashes.json"})

    def rewrite_summary(self, mutate):
        path = self.archive / "results.json"
        summary = json.loads(path.read_text())
        mutate(summary)
        write_json(path, summary)
        self.rehash()

    def test_published_evidence_retains_all_six_assignments(self):
        result = verify(self.archive)
        self.assertEqual(result["assigned"], 6)
        self.assertTrue(result["integrity_verified"])

    def test_rehashed_aggregate_cannot_hide_missing_trial_or_inflate_success(self):
        self.rewrite_summary(lambda s: s["conditions"]["C"].update(
            success_at_180_seconds={"numerator": 3, "denominator": 3}))
        with self.assertRaisesRegex(ValueError, "denominators"):
            verify(self.archive)

    def test_missing_unassessed_trial_is_rejected(self):
        self.rewrite_summary(lambda s: s.update(trials=[
            row for row in s["trials"] if row["status"] != "unassessed"]))
        with self.assertRaisesRegex(ValueError, "Assigned trials"):
            verify(self.archive)

    def test_rehashed_submission_still_needs_the_assessed_digest(self):
        summary = json.loads((self.archive / "results.json").read_text())
        subject = next(row for row in summary["trials"] if row["task"] == "sampling-dev"
                       and row["condition"] == "A")
        path = self.archive / subject["id"] / "solution.in"
        path.write_text(path.read_text() + "# changed after assessment\n")
        self.rehash()
        with self.assertRaisesRegex(ValueError, "another submission"):
            verify(self.archive)

    def test_unreliable_clock_interval_cannot_be_promoted_to_execution_time(self):
        def mutate(summary):
            subject = next(row for row in summary["trials"] if row["task"] == "orientation-dev"
                           and row["condition"] == "C")
            subject["agent_seconds"] = subject["harbor_timestamp_interval_seconds"]
            subject["agent_seconds_reliable"] = True
        self.rewrite_summary(mutate)
        with self.assertRaisesRegex(ValueError, "wall timestamps"):
            verify(self.archive)

    def test_changed_frozen_oracle_fails_even_with_new_outer_hashes(self):
        path = self.archive / "sources/tools/repair_tasks.py"
        path.write_text(path.read_text() + "\n# modified grader\n")
        self.rehash()
        with self.assertRaisesRegex(ValueError, "Frozen file changed"):
            verify(self.archive)


if __name__ == "__main__":
    unittest.main()

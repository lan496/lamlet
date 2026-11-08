import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from p3_regrade import verify_archive


ROOT = Path(__file__).resolve().parents[1]


class RegradeTests(unittest.TestCase):
    def test_archive_verification_rejects_modified_submission(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary)
            source = archive / "solution.in"
            source.write_bytes(b"run 100000\n")
            (archive / "hashes.json").write_text(json.dumps({
                "solution.in": hashlib.sha256(source.read_bytes()).hexdigest()}))
            verify_archive(archive)
            source.write_bytes(b"run 2\n")
            with self.assertRaisesRegex(ValueError, "Frozen archive changed"):
                verify_archive(archive)

    def test_published_regrade_preserves_original_inputs_and_missing_trials(self):
        original = ROOT / "docs/assets/evaluation/2026-11-08"
        revised = ROOT / "docs/assets/evaluation/2026-11-08-protocol-v2"
        verify_archive(original)
        verify_archive(revised)
        report = json.loads((revised / "report.json").read_text())
        self.assertTrue(report["post_hoc"])
        self.assertFalse(report["model_trials_run"])
        self.assertFalse(report["submissions_modified"])
        records = {trial["id"]: trial for trial in report["trials"]}
        for name in ("A1", "A2", "C1", "C2"):
            record = records[name]
            self.assertEqual(record["submission_sha256"],
                hashlib.sha256((original / name / "solution.in").read_bytes()).hexdigest())
            self.assertEqual(record["original_status"], "unassessed")
            self.assertEqual(record["scientific_status"], "correct")
            assessment = json.loads((revised / record["assessment"]).read_text())
            self.assertEqual({f["status"] for f in assessment["oracle"]["criteria"].values()}, {"pass"})
        for name in ("A3", "C3"):
            self.assertFalse(records[name]["input_submitted"])
            self.assertEqual(records[name]["scientific_status"], "unassessed")
        for condition in ("A", "C"):
            self.assertEqual(report["conditions"][condition]["assigned"], 3)
            self.assertEqual(report["conditions"][condition]["correct"], 2)
            self.assertEqual(report["conditions"][condition]["unassessed"], 1)


if __name__ == "__main__":
    unittest.main()

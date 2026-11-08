"""Check admission evidence without requiring Docker or asserting physics."""

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from corpus import write_json
import p3_harbor_admit
from p3_harbor_admit import assessment_checks, digest, publish, verify_files
from p3_oracle import VERSION


class HarborAdmissionTests(unittest.TestCase):
    def trial(self, root, status):
        trial = root / "trial"
        payload = {"schema_version": 1, "task": "P3", "submission_sha256": "input-hash",
                   "oracle_version": VERSION, "criteria": {
                       "gap": {"status": status, "evidence": ["transport fixture only"]}},
                   "execution": {"full_equilibration_executed": False}}
        grade = {"pass": "correct", "fail": "incorrect", "unknown": "unassessed"}[status]
        reward = {"pass": 1, "fail": 0, "unknown": None}[status]
        write_json(trial / "verifier/assessment.json", {
            "scientific_status": grade, "reward": reward, "submission_sha256": "input-hash",
            "oracle_sha256": "oracle-hash", "oracle": payload})
        write_json(trial / "result.json", {
            "verifier_environment_mode": "separate",
            "verifier_result": None if reward is None else {"rewards": {"reward": reward}},
            "exception_info": {"exception_type": "RewardFileNotFoundError"} if reward is None else None})
        (trial / "agent").mkdir()
        (trial / "agent/oracle.txt").write_text("agent phase: verifier files absent\n")
        if reward is not None:
            (trial / "verifier/reward.txt").write_text(str(reward) + "\n")
        return trial, grade

    def check(self, trial, grade, criterion=None):
        return assessment_checks(trial, grade, criterion, "input-hash", "oracle-hash",
                                 {"id": "P3", "criteria": ["gap"]})

    def test_independent_pass_and_fail_require_matching_harbor_rewards(self):
        for status, reward in (("pass", 1), ("fail", 0)):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as temporary:
                trial, grade = self.trial(Path(temporary), status)
                self.assertTrue(self.check(trial, grade, "gap" if status == "fail" else None)["passed"])
                (trial / "verifier/reward.txt").write_text(str(1 - reward))
                observed = self.check(trial, grade)
                self.assertFalse(observed["passed"])
                self.assertFalse(observed["checks"]["forged_artifacts_ignored"])

    def test_unknown_requires_absent_rewards_and_the_specific_harbor_exception(self):
        with tempfile.TemporaryDirectory() as temporary:
            trial, grade = self.trial(Path(temporary), "unknown")
            self.assertTrue(self.check(trial, grade)["passed"])
            (trial / "verifier/reward.json").write_text('{"reward":1}')
            self.assertFalse(self.check(trial, grade)["passed"])
            (trial / "verifier/reward.json").unlink()
            path = trial / "result.json"
            result = json.loads(path.read_text())
            result["exception_info"]["exception_type"] = "EnvironmentStartError"
            write_json(path, result)
            self.assertFalse(self.check(trial, grade)["passed"])

    def test_stale_oracle_wrong_submission_and_wrong_target_are_rejected(self):
        mutations = {
            "oracle_identity": lambda p: p.update(oracle_sha256="stale"),
            "submission_identity": lambda p: p.update(submission_sha256="different-input"),
            "adapter_agreement": lambda p: p.update(infrastructure_error="engine crashed"),
            "bounded_smoke": lambda p: p["oracle"]["execution"].update(full_equilibration_executed=True),
        }
        for check, mutate in mutations.items():
            with self.subTest(check=check), tempfile.TemporaryDirectory() as temporary:
                trial, grade = self.trial(Path(temporary), "fail")
                path = trial / "verifier/assessment.json"
                payload = json.loads(path.read_text())
                mutate(payload)
                write_json(path, payload)
                observed = self.check(trial, grade, "gap")
                self.assertFalse(observed["passed"])
                self.assertFalse(observed["checks"][check])
        with tempfile.TemporaryDirectory() as temporary:
            trial, grade = self.trial(Path(temporary), "pass")
            self.assertFalse(self.check(trial, grade, "gap")["checks"]["target_criterion"])

    def test_shared_environment_and_missing_agent_assertion_cannot_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            trial, grade = self.trial(Path(temporary), "pass")
            path = trial / "result.json"
            payload = json.loads(path.read_text())
            payload["verifier_environment_mode"] = "shared"
            write_json(path, payload)
            self.assertFalse(self.check(trial, grade)["passed"])
            payload["verifier_environment_mode"] = "separate"
            write_json(path, payload)
            (trial / "agent/oracle.txt").write_text("submission copied without isolation checks")
            self.assertFalse(self.check(trial, grade)["passed"])

    def test_modified_missing_or_empty_frozen_inputs_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "task.json"
            path.write_text('{"criteria":["gap"]}')
            hashes = {"task.json": digest(path)}
            verify_files(root, hashes)
            path.write_text('{"criteria":[]}')
            with self.assertRaisesRegex(ValueError, "Frozen admission file changed"):
                verify_files(root, hashes)
            path.unlink()
            with self.assertRaises(ValueError):
                verify_files(root, hashes)
            with self.assertRaises(ValueError):
                verify_files(root, {})

    def test_timing_subsets_and_incomplete_runs_cannot_be_published(self):
        for complete, selection, passed in ((False, True, False), (True, False, True), (True, True, False)):
            with self.subTest(complete=complete, selection=selection, passed=passed), \
                    tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                write_json(root / "report.json", {"complete": complete, "complete_selection": selection,
                                                 "harbor_admission_passed": passed})
                destination = root / "public"
                with self.assertRaisesRegex(ValueError, "complete successful"):
                    publish(root, destination)
                self.assertFalse(destination.exists())

    def test_publication_keeps_verifiable_evidence_and_rejects_a_changed_input(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "output"
            trial, grade = self.trial(output, "pass")
            write_json(root / "research/harbor/tasks.json", {"tasks": {"P3": {"criteria": ["gap"]}}})
            oracle = root / "tools/p3_oracle.py"
            oracle.parent.mkdir()
            oracle.write_text("transport fixture, not executable")
            input_path = output / "reference/tasks/nb-impact/solution/reference.in"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("run 0\n")
            assessment_path = trial / "verifier/assessment.json"
            assessment = json.loads(assessment_path.read_text())
            assessment.update(oracle_sha256=digest(oracle), submission_sha256=digest(input_path))
            assessment["oracle"]["submission_sha256"] = digest(input_path)
            write_json(assessment_path, assessment)
            record = {"case": "reference", "trial": "trial", "expected": grade, "criterion": None,
                      "submission_sha256": digest(input_path), "passed": True}
            report = {"complete": True, "complete_selection": True, "harbor_admission_passed": True,
                      "sources": {name: digest(root / name) for name in
                                  ("research/harbor/tasks.json", "tools/p3_oracle.py")}, "cases": [record]}
            write_json(output / "report.json", report)
            write_json(output / "plan.json", {"cases": [record]})
            destination = root / "public"
            with patch.object(p3_harbor_admit, "ROOT", root):
                publish(output, destination)
                archived = json.loads((destination / "report.json").read_text())
                archived_case = next(c for c in archived["cases"] if c["case"] == "reference")
                self.assertEqual(archived_case["trial"], "reference")
                self.assertEqual((destination / "reference/solution.in").read_text(), "run 0\n")
                self.assertEqual(json.loads((destination / "reference/verifier/assessment.json").read_text()),
                                 assessment)
                verify_files(destination, json.loads((destination / "hashes.json").read_text()))
                input_path.write_text("run 1000\n")
                with self.assertRaisesRegex(ValueError, "submission changed"):
                    publish(output, root / "changed")
                self.assertFalse((root / "changed").exists())


if __name__ == "__main__":
    unittest.main()

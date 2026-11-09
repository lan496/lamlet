import copy
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import defect_diagnostics as audit


ARCHIVE = audit.ROOT / "docs/assets/defect-diagnostics/2026-11-09"
GCMC_ARCHIVE = audit.ROOT / "docs/assets/defect-diagnostics/2026-11-09-gcmc"
REVIEW = audit.ROOT / "docs/assets/defect-diagnostics/review"


class NaturalDiagnosticTests(unittest.TestCase):
    def test_frozen_native_conditions_preserve_timeouts_and_nonreproduction(self):
        plan, native, config = audit.context()
        states = {}
        for case in plan["cases"]:
            source = next(c for c in native["cases"] if c["id"] == case["id"])
            for variant in source["variants"]:
                at = audit.ROOT / plan["native_archive"] / case["id"] / variant["variant"]
                states[case["id"] + "/" + variant["variant"]] = audit.reference(
                    case, variant, config, (at / "engine.stdout").read_text())
        self.assertEqual(states, {
            "orientation/before": "violated", "orientation/after": "satisfied",
            "averaging/before": "execution_only", "averaging/after": "execution_only",
            "gcmc-original/before": "unassessed", "gcmc-original/after": "unassessed",
            "gcmc-instrumented/before": "violated", "gcmc-instrumented/after": "satisfied",
            "velocity/before": "violated", "velocity/after": "satisfied"})

    def test_changed_evidence_and_reduced_plan_fail_before_checker_execution(self):
        original = json.loads(audit.PLAN.read_bytes())
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "plan.json"
            for mutation in ("hash", "missing", "duplicate", "target"):
                plan = copy.deepcopy(original)
                case = next(c for c in plan["cases"] if c["id"] == "velocity")
                if mutation == "hash":
                    plan["native_report_sha256"] = "stale"
                elif mutation == "missing":
                    plan["cases"].remove(case)
                elif mutation == "duplicate":
                    plan["cases"].append(case)
                else:
                    case["expected_target"]["before"] = "violated"
                path.write_text(json.dumps(plan))
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    audit.context(path)

    def test_targets_select_the_named_temperature_instead_of_another_compute(self):
        case = dict(target_rule="LML-PHYS-010", target_subject="fix NVT temperature selection")
        report = {"summary": {"errors": 0}, "diagnostics": [
            {"code": "LML-PHYS-010", "severity": "warning", "evidence": [{"subject": "thermo temp"}]}],
            "physical_findings": [
                {"rule_id": "LML-PHYS-010", "outcome": "violated", "evidence": [{"subject": "thermo temp"}]},
                {"rule_id": "LML-PHYS-010", "outcome": "unknown", "evidence": [{"subject": case["target_subject"]}]}]}
        modes = {mode: {"status": "collected", "report": report} for mode in audit.MODES}
        result = audit.observation(case, modes, "same hash", "same hash")
        self.assertEqual(result["target_outcomes"], ["unknown"])
        self.assertEqual(result["target_state"], "unknown")
        self.assertFalse(result["target_warned"])
        self.assertEqual(result["warnings"], ["LML-PHYS-010"])
        self.assertEqual(result["unknown_rules"], ["LML-PHYS-010"])
        self.assertTrue(all(result["legacy"].values()))
        modes["check"] = {"status": "infrastructure_failure", "error": "fixture crash"}
        failed = audit.observation(case, modes, None, "original hash")
        self.assertFalse(failed["assessed"])
        self.assertEqual(failed["target_state"], "unavailable")
        self.assertFalse(failed["legacy"]["byte_identical"])
        self.assertFalse(failed["legacy"]["diagnostics_agree"])

    def test_missing_measurement_or_unrelated_engine_error_stays_unassessed(self):
        self.assertEqual(audit.reference(dict(basis="command_acceptance"),
            dict(timed_out=False, engine_exit_code=1), {}, "Unrelated engine failure"), "unassessed")
        variant = dict(timed_out=False, engine_exit_code=0, measurement=None, measurement_error="missing")
        self.assertEqual(audit.reference(dict(basis="kinetic_energy"), variant, {}, ""), "unassessed")

    def test_path_normalization_preserves_spans_and_subjects(self):
        path = Path("/workspace/input.in")
        data = {"span": {"source": str(path), "line": 17}, "evidence": ["fix NVT temperature selection"]}
        self.assertEqual(audit.normalize(data, path), {"span": {"source": "inputs/input.in", "line": 17},
                         "evidence": ["fix NVT temperature selection"]})

    def test_public_archive_recomputes_all_ten_observations(self):
        result = audit.verify(ARCHIVE)
        self.assertEqual(result["verified_inputs"], 10)
        self.assertFalse(result["checker_rerun"])
        self.assertFalse(result["source_inputs_verified_offline"])
        report = json.loads((ARCHIVE / "report.json").read_bytes())
        self.assertEqual(report["summary"]["reference_statuses"],
                         dict(violated=3, satisfied=3, execution_only=2, unassessed=2))
        observed = {case["id"]: case["observed"] for case in report["cases"]}
        self.assertEqual({name: (result["target_state"], result["target_warned"])
                          for name, result in observed.items() if name.startswith("gcmc-")}, {
            "gcmc-original/before": ("absent", False), "gcmc-original/after": ("absent", False),
            "gcmc-instrumented/before": ("absent", False), "gcmc-instrumented/after": ("absent", False)})
        self.assertEqual(observed["orientation/before"]["target_state"], "violated")
        self.assertTrue(observed["orientation/before"]["target_warned"])
        self.assertEqual(observed["orientation/after"]["target_state"], "satisfied")
        self.assertFalse(observed["orientation/after"]["target_warned"])
        self.assertTrue(all(all(result["legacy"].values()) for result in observed.values()))
        self.assertEqual(report["gold_defect_pairs"], 0)
        self.assertIsNone(report["checker_accuracy"])

    def test_archive_rejects_dropped_case_and_recomputed_receipt_on_false_summary(self):
        with tempfile.TemporaryDirectory() as temporary:
            at = Path(temporary) / "archive"
            shutil.copytree(ARCHIVE, at)
            report_path = at / "report.json"
            report = json.loads(report_path.read_bytes())
            report["summary"]["mapped_violated_warned"] = 100
            report_path.write_text(json.dumps(report))
            inventory_path = at / "inventory.json"
            inventory = json.loads(inventory_path.read_bytes())
            inventory["report.json"] = audit.digest(report_path)
            inventory_path.write_text(json.dumps(inventory))
            with self.assertRaisesRegex(ValueError, "summary differs"):
                audit.verify(at)
            (at / "diagnostics/orientation/before/check.json").unlink()
            with self.assertRaisesRegex(ValueError, "inventory"):
                audit.verify(at)

    def test_gcmc_regrade_preserves_inputs_native_outcomes_and_historical_capture(self):
        self.assertEqual(audit.verify(GCMC_ARCHIVE)["verified_inputs"], 10)
        old = json.loads((ARCHIVE / "report.json").read_bytes())
        new = json.loads((GCMC_ARCHIVE / "report.json").read_bytes())
        self.assertEqual((ARCHIVE / "plan.json").read_bytes(),
                         (GCMC_ARCHIVE / "plan.json").read_bytes())
        previous = {case["id"]: case for case in old["cases"]}
        current = {case["id"]: case for case in new["cases"]}
        self.assertEqual(set(previous), set(current))
        for identity, case in current.items():
            with self.subTest(identity=identity):
                self.assertEqual(case["input_sha256"], previous[identity]["input_sha256"])
                self.assertEqual(case["reference_status"], previous[identity]["reference_status"])
                self.assertTrue(all(case["observed"]["legacy"].values()))
                if identity.startswith("gcmc-"):
                    self.assertEqual(previous[identity]["observed"]["target_state"], "absent")
                    before = identity.endswith("/before")
                    self.assertEqual(case["observed"]["target_state"], "violated" if before else "satisfied")
                    self.assertEqual(case["observed"]["target_warned"], before)
                else:
                    self.assertEqual(case["observed"]["target_state"], previous[identity]["observed"]["target_state"])
                    self.assertEqual(case["observed"]["target_warned"], previous[identity]["observed"]["target_warned"])
        self.assertNotEqual(new["binary"]["declared_source_commit"], old["binary"]["declared_source_commit"])
        self.assertNotEqual(new["binary"]["sha256"], old["binary"]["sha256"])
        self.assertEqual(new["summary"]["declared_targets_matched"], 6)
        self.assertEqual(new["summary"]["mapped_violated_warned"], 2)
        self.assertEqual(new["summary"]["reference_statuses"], old["summary"]["reference_statuses"])
        self.assertEqual(new["gold_defect_pairs"], 0)
        self.assertEqual(new["independent_review"], "pending")
        self.assertIsNone(new["checker_accuracy"])
        self.assertEqual(new["new_lammps_launches"], 0)
        self.assertEqual(new["agent_trials"], 0)


class NaturalReviewTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.at = Path(temporary.name) / "review"
        audit.prepare_review(self.at)
        self.template = json.loads((self.at / "review.json").read_bytes())

    def test_packet_is_reproducible_pending_and_excludes_checker_findings(self):
        for name in ("packet.md", "review.json"):
            self.assertEqual((self.at / name).read_bytes(), (REVIEW / name).read_bytes())
        packet = (self.at / "packet.md").read_text()
        for case in self.template["cases"]:
            self.assertIn("## " + case["id"] + "\n", packet)
        self.assertIn("source_fragment_with_reconstructed_correction", packet)
        self.assertIn("timed_out", packet)
        self.assertNotIn("LML-", packet)
        self.assertNotIn("physical_findings", packet)
        result = audit.validate_review(self.at / "review.json", self.template)
        self.assertEqual(result["decisions"], {"pending": 5})
        self.assertFalse(result["all_conditions_agreed"])
        process = subprocess.run([sys.executable, str(audit.ROOT / "tools/defect_diagnostics.py"),
            "validate-review", "--review", str(self.at / "review.json")], capture_output=True)
        self.assertEqual(process.returncode, 1)

    def test_attestation_never_verifies_identity_or_admits_gold_pairs(self):
        changed = copy.deepcopy(self.template)
        for case in changed["cases"]:
            case.update(decision="agree", reason="Test fixture only.")
        path = self.at / "filled.json"
        path.write_text(json.dumps(changed))
        self.assertFalse(audit.validate_review(path, self.template)["all_conditions_agreed"])
        changed["reviewer"] = dict(name="Test fixture", affiliation="Unit test", date="2026-11-09", independent=True)
        path.write_text(json.dumps(changed))
        result = audit.validate_review(path, self.template)
        self.assertTrue(result["all_conditions_agreed"])
        self.assertFalse(result["identity_verified_by_tool"])
        self.assertEqual(result["gold_pairs_admitted_by_tool"], 0)

    def test_stale_missing_duplicate_and_unreasoned_decisions_fail(self):
        for mutation in ("stale", "missing", "duplicate", "reason", "decision", "packet"):
            form = copy.deepcopy(self.template)
            case = next(c for c in form["cases"] if c["id"] == "velocity")
            if mutation == "stale":
                case["input_sha256"]["before"] = "changed"
            elif mutation == "missing":
                form["cases"].remove(case)
            elif mutation == "duplicate":
                form["cases"].append(case)
            elif mutation == "reason":
                case["decision"] = "agree"
            elif mutation == "packet":
                form["packet_sha256"] = "changed"
            else:
                case["decision"] = "gold"
            path = self.at / "changed.json"
            path.write_text(json.dumps(form))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                audit.validate_review(path, self.template)


if __name__ == "__main__":
    unittest.main()

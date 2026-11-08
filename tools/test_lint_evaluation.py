"""Check fixed denominators and failure handling independently of lint logic."""

import copy
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import lint_evaluation as lint


class LintEvaluationTests(unittest.TestCase):
    def test_missing_and_modified_inputs_fail_even_outside_selected_case(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "lint"
            shutil.copytree(lint.CORPUS.parent, directory)
            manifest = directory / "corpus.json"
            victim = directory / "inputs/drag-zero.in"
            victim.write_bytes(b"fix ensemble all nvt temp 300 300 0.1 drag 1\n")
            with self.assertRaisesRegex(ValueError, "Modified input: drag-zero"):
                lint.evaluate(Path("not-invoked"), manifest, "averaging-boundary")
            victim.unlink()
            with self.assertRaises(FileNotFoundError):
                lint.evaluate(Path("not-invoked"), manifest, "averaging-boundary")

    def test_stale_provenance_and_mutations_fail_before_cli_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "lint"
            shutil.copytree(lint.CORPUS.parent, directory)
            manifest = directory / "corpus.json"
            original = json.loads(manifest.read_bytes())
            spec = copy.deepcopy(original)
            reference = spec["rules"]["LML-MAN-AVE-TIME"]["sources"]
            next(r for r in reference if r["path"] == "fix_ave_time.rst")["sha256"] = "stale"
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "Stale manual source"):
                lint.load_cases(manifest)
            spec = copy.deepcopy(original)
            case = next(c for c in spec["cases"] if c["id"] == "averaging-window")
            case["mutation"]["after"] = "ave/time 2 7 10"
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "Stale single-edit mutation"):
                lint.load_cases(manifest)

    def test_omitted_or_duplicate_cases_cannot_change_denominators_silently(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "lint"
            shutil.copytree(lint.CORPUS.parent, directory)
            manifest = directory / "corpus.json"
            original = json.loads(manifest.read_bytes())
            spec = copy.deepcopy(original)
            spec["cases"] = [c for c in spec["cases"] if c["id"] != "drag-zero"]
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "incomplete corpus"):
                lint.load_cases(manifest)
            # Remove an unreferenced control to reach the file-inventory check.
            spec = copy.deepcopy(original)
            spec["cases"] = [c for c in spec["cases"] if c["id"] != "drag-default"]
            spec["case_count"] = len(spec["cases"])
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "Unlisted corpus inputs"):
                lint.load_cases(manifest)
            spec = copy.deepcopy(original)
            spec["cases"].append(next(c for c in spec["cases"] if c["id"] == "drag-zero"))
            spec["case_count"] = len(spec["cases"])
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "Duplicate IDs"):
                lint.load_cases(manifest)
            spec["cases"] = []
            spec["case_count"] = 0
            manifest.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError, "incomplete corpus"):
                lint.load_cases(manifest)

    def test_complete_diagnostic_hash_is_portable_and_detects_text_changes(self):
        def report(path):
            return {"file": str(path), "summary": {"errors": 0}, "diagnostics": [
                {"file": str(path), "code": "U_TEST", "severity": "unknown",
                 "message": "Unverified", "span": {"start_line": 1}}]}

        first, second = Path("one/input.in"), Path("two/input.in")
        observed = lint.observe(report(first), "target", first)
        portable = lint.observe(report(second), "target", second)
        self.assertEqual(observed["diagnostics_sha256"], portable["diagnostics_sha256"])
        changed = report(second)
        diagnostic = next(d for d in changed["diagnostics"] if d["code"] == "U_TEST")
        diagnostic["message"] = "Changed explanation"
        self.assertNotEqual(observed["diagnostics_sha256"],
                            lint.observe(changed, "target", second)["diagnostics_sha256"])

    def test_missed_finding_false_warning_unknown_and_rewritten_bytes_fail(self):
        _, cases = lint.load_cases(lint.CORPUS)
        for identity, outcomes, warnings, expected_pass, rewrite, strict_error in [
            ("drag-nonzero", [], [], False, False, False),
            ("drag-zero", ["violated"], ["LML-MAN-NH-DRAG"], False, False, False),
            ("drag-runtime", [], [], False, False, False),
            ("drag-zero", ["satisfied"], [], False, True, False),
            ("drag-zero", ["satisfied"], [], False, False, True),
            ("drag-nonzero", ["violated"], ["LML-MAN-NH-DRAG"], True, False, False),
            ("integration-sequential", [], [], True, False, False),
        ]:
            case = next(c for c in cases if c["id"] == identity)
            report = {"summary": {"errors": 0}, "diagnostics": [
                {"code": code, "severity": "warning"} for code in warnings]}
            if outcomes:
                report["physical_findings"] = [
                    {"rule_id": case["rule_id"], "outcome": outcome} for outcome in outcomes]

            def findings(binary, path, output=None, strict=False):
                result = copy.deepcopy(report)
                if output:
                    output.write_bytes(b"# changed\n" if rewrite else path.read_bytes())
                if strict and strict_error:
                    result["summary"]["errors"] = 1
                    result["diagnostics"].append({"code": "E_NEW", "severity": "error"})
                return result

            with self.subTest(identity=identity, rewrite=rewrite, strict_error=strict_error):
                with tempfile.TemporaryDirectory() as temporary:
                    with patch.object(lint, "findings", side_effect=findings):
                        result = lint.evaluate_case(Path("unused"), lint.CORPUS.parent, case,
                                                    Path(temporary) / "output.in")
                self.assertEqual(result["passed"], expected_pass)

    def test_metrics_keep_misses_false_warnings_unknown_and_absence_separate(self):
        legacy = dict.fromkeys(("normal_accepted", "strict_accepted", "build_accepted",
                                "byte_identical", "diagnostics_agree"), True)
        cases = []
        for label, outcomes, warnings, passed in [
            ("positive", [], [], False),
            ("positive", ["violated"], ["target"], True),
            ("negative", ["satisfied"], ["other-rule"], False),
            ("negative", [], [], True),
            ("unknown", ["unknown"], [], True),
            ("out_of_scope", [], [], True),
        ]:
            cases.append({"rule_id": "target", "label": label, "passed": passed,
                          "observed": {"outcomes": outcomes, "warnings": warnings},
                          "legacy": legacy})
        self.assertEqual(lint.summarize(cases), {
            "cases": 6, "passed": 4,
            "detection": {"numerator": 1, "denominator": 2, "rate": 0.5},
            "false_warning": {"numerator": 1, "denominator": 2, "rate": 0.5},
            "unknown": {"numerator": 1, "denominator": 6, "rate": 1 / 6},
            "unknown_recognition": {"numerator": 1, "denominator": 1, "rate": 1.0},
            "unknown_warned": 0, "not_evaluated": 3,
            "legacy": dict.fromkeys(legacy, 6),
        })
        self.assertIsNone(lint.summarize([])["detection"]["rate"])

    def test_changed_observations_fail_baseline_even_when_executable_changes(self):
        baseline = {"binary": {"sha256": "linux"}, "summary": {"passed": 1}}
        report = {"binary": {"sha256": "macos"}, "summary": {"passed": 1}}
        self.assertTrue(lint.matches_baseline(report, baseline))
        report["summary"]["passed"] = 0
        self.assertFalse(lint.matches_baseline(report, baseline))

    def test_checker_crashes_remain_infrastructure_failures(self):
        process = subprocess.CompletedProcess([], -9, b"", b"killed")
        with patch.object(lint.subprocess, "run", return_value=process):
            with self.assertRaisesRegex(ValueError, "CLI failed"):
                lint.evaluate_case(Path("lamlet"), lint.CORPUS.parent,
                                   {"id": "drag-zero", "rule_id": "LML-MAN-NH-DRAG"},
                                   Path("unused"))

    def test_documented_counts_match_archived_observations(self):
        report = json.loads((lint.ROOT / "docs/assets/lint-evaluation/report.json").read_bytes())
        page = (lint.ROOT / "docs/lint-evaluation.md").read_text()
        rows = re.findall(r"\| `(LML-[A-Z0-9-]+)` \| (\d+/\d+) \| (\d+/\d+) \| (\d+/\d+) \|", page)
        observed = {identity: (detection, false_warning, unknown)
                    for identity, detection, false_warning, unknown in rows}
        fraction = lambda metric: f"{metric['numerator']}/{metric['denominator']}"
        expected = {identity: tuple(fraction(summary[key]) for key in
                                    ("detection", "false_warning", "unknown"))
                    for identity, summary in report["by_rule"].items()}
        self.assertEqual(observed, expected)


if __name__ == "__main__":
    unittest.main()

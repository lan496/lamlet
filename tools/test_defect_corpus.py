import copy
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import defect_corpus as corpus
import defect_replay as replay


class DefectCollectionTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest = self.root / "cases.json"
        self.case = dict(id="m123", url="https://matsci.org/t/example/123", family="test",
                         disposition="proposed_fix", review="pending", split="development",
                         intent="Observe the intended value.", condition="A known test condition.",
                         reason="Test evidence only.", blockers=["Independent review pending."],
                         evidence=[dict(post=2, anchor="The correction", role="correction")])
        self.manifest.write_text(json.dumps(dict(schema_version=1, cases=[self.case])))
        self.first = dict(id=10, post_number=1, username="author", created_at="date",
                          updated_at="date", cooked="<p>The report.</p><pre><code>run 0\n</code></pre>")
        self.last = dict(id=20, post_number=2, username="responder", created_at="date",
                         updated_at="date", cooked="<p>The correction.</p>")

    def collect(self, attachments=False):
        if attachments:
            self.last["cooked"] += '<a class="attachment" href="/uploads/data.in">input</a>'
        topic = dict(id=123, title="Example", post_stream=dict(posts=[self.first], stream=[10, 20]))

        def read(url):
            if url.endswith("example/123.json"):
                return json.dumps(topic).encode()
            if "posts.json?" in url:
                return json.dumps(dict(post_stream=dict(posts=[self.last]))).encode()
            if url.endswith("data.in"):
                return b"run 0\n"
            self.fail(f"Unexpected source request: {url}")

        with patch.object(corpus, "read_url", side_effect=read):
            return corpus.fetch(self.manifest, self.root / "cache")

    def test_fetch_captures_later_posts_and_verifies_cited_content(self):
        lock = self.collect(attachments=True)
        report = corpus.audit(self.manifest, lock, self.root / "cache")
        self.assertEqual(report["screened"], 1)
        self.assertEqual(report["captured"], 1)
        self.assertEqual(report["downloaded_attachments"], 1)
        self.assertEqual(report["gold_defect_pairs"], 0)
        self.assertFalse(report["checker_evaluated"])
        self.assertEqual(report["dispositions"], {"proposed_fix": 1})
        with self.assertRaisesRegex(ValueError, "fresh snapshot"):
            corpus.fetch(self.manifest, self.root / "cache")

    def test_missing_or_duplicate_posts_fail_instead_of_reducing_denominator(self):
        topic = dict(post_stream=dict(posts=[self.first], stream=[10, 20]))
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            corpus.complete_posts(topic)
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            corpus.complete_posts(topic, [self.first, self.last])

    def test_failed_capture_is_retained(self):
        with patch.object(corpus, "read_url", side_effect=OSError("fixture unavailable")):
            lock = corpus.fetch(self.manifest, self.root / "cache")
        report = corpus.audit(self.manifest, lock)
        self.assertEqual(report["screened"], 1)
        self.assertEqual(report["captured"], 0)
        self.assertEqual(report["capture_failures"], 1)

    def test_changed_source_bytes_and_empty_evidence_fail(self):
        lock = self.collect()
        path = self.root / "cache/m123/topic.json"
        path.write_bytes(path.read_bytes() + b" ")
        with self.assertRaisesRegex(ValueError, "Changed source snapshot"):
            corpus.audit(self.manifest, lock, self.root / "cache")
        self.case["evidence"].clear()
        self.manifest.write_text(json.dumps(dict(schema_version=1, cases=[self.case])))
        with self.assertRaisesRegex(ValueError, "Missing screening evidence"):
            corpus.audit(self.manifest, lock)

    def test_missing_sources_stale_selection_and_false_gold_status_fail(self):
        lock = self.collect()
        for mutation in ("missing", "selection", "review", "receipt", "anchor"):
            changed = copy.deepcopy(lock)
            case = copy.deepcopy(self.case)
            if mutation == "missing":
                changed["sources"].clear()
            elif mutation == "selection":
                changed["selection_sha256"] = "stale"
            elif mutation == "review":
                case["review"] = "independently_verified"
            elif mutation == "receipt":
                source = next(s for s in changed["sources"] if s["id"] == "m123")
                receipt = next(p for p in source["posts"] if p["post_number"] == 2)
                receipt["cooked_sha256"] = "0" * 64
            else:
                case["evidence"] = [dict(post=2, anchor="Not in the source", role="claim")]
            self.manifest.write_text(json.dumps(dict(schema_version=1, cases=[case])))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                corpus.audit(self.manifest, changed, self.root / "cache")

    def test_public_audit_is_deterministic_and_keeps_all_screened_sources(self):
        command = [sys.executable, str(corpus.ROOT / "tools/defect_corpus.py"), "audit"]
        first = subprocess.run(command, capture_output=True, check=True)
        second = subprocess.run(command, capture_output=True, check=True)
        self.assertEqual(first.stdout, second.stdout)
        report = json.loads(first.stdout)
        self.assertEqual(report["screened"], 20)
        self.assertEqual(report["dispositions"], {"counterexample": 2, "engine_bug": 1,
                         "proposed_fix": 7, "reported_fix": 6, "unresolved": 4})
        self.assertEqual(report["gold_defect_pairs"], 0)
        self.assertEqual(report["held_out_cases"], 0)
        archived = json.loads((corpus.ROOT / "research/defects/collection.json").read_text())
        # The published receipt includes a cache-backed check performed at collection.
        self.assertTrue(archived.pop("cached_bytes_verified"))
        report.pop("cached_bytes_verified")
        self.assertEqual(report, archived)


class DefectReplayTests(unittest.TestCase):
    def test_published_archive_recomputes_measurements_and_retains_timeouts(self):
        archive = corpus.ROOT / "docs/assets/defect-corpus/2026-11-09"
        command = [sys.executable, str(corpus.ROOT / "tools/defect_replay.py"),
                   "verify", "--run", str(archive)]
        process = subprocess.run(command, capture_output=True, check=True)
        self.assertEqual(json.loads(process.stdout), dict(verified_cases=5, verified_files=24,
                         source_transformations_verified=False))
        report = json.loads((archive / "report.json").read_text())
        full = next(case for case in report["cases"] if case["id"] == "gcmc-original")
        self.assertEqual({v["variant"] for v in full["variants"] if v["timed_out"]}, {"before", "after"})

    def test_archive_rejects_missing_output_and_changed_measurements(self):
        archive = corpus.ROOT / "docs/assets/defect-corpus/2026-11-09"
        with tempfile.TemporaryDirectory() as temporary:
            copy_path = Path(temporary) / "archive"
            shutil.copytree(archive, copy_path)
            output = copy_path / "velocity/before/engine.stdout"
            output.unlink()
            with self.assertRaisesRegex(ValueError, "inventory"):
                replay.verify(copy_path)
            shutil.copyfile(archive / "velocity/before/engine.stdout", output)
            report_path = copy_path / "report.json"
            report = json.loads(report_path.read_text())
            velocity = next(case for case in report["cases"] if case["id"] == "velocity")
            before = next(v for v in velocity["variants"] if v["variant"] == "before")
            before["measurement"]["kinetic_energy_eV"] = 95.268360648
            report_path.write_text(json.dumps(report))
            inventory_path = copy_path / "inventory.json"
            inventory = json.loads(inventory_path.read_text())
            inventory["report.json"] = corpus.sha256(report_path.read_bytes())
            inventory_path.write_text(json.dumps(inventory))
            with self.assertRaisesRegex(ValueError, "Changed independent measurement"):
                replay.verify(copy_path)

    def test_kinetic_energy_is_checked_against_an_independent_expected_value(self):
        case = dict(id="velocity", mass=2.0, velocity=3.0)
        config = dict(metal_mvv2e=0.5)
        measured = replay.measurement(case, config, "Step c_measured\n0 4.5\nLoop time\n")
        self.assertEqual(measured, dict(kinetic_energy_eV=4.5, intended_energy_eV=4.5, relative_error=0.0))
        wrong = replay.measurement(case, config, "Step c_measured\n0 9\nLoop time\n")
        self.assertEqual(wrong["relative_error"], 1.0)

    def test_temperature_normalization_uses_current_atom_count(self):
        text = "Step Atoms KinEng c_NVT_temp c_Tfluid\n0 2 3 2 2\n100 3 6 4 2\nLoop time\n"
        result = replay.measurement(dict(id="gcmc-instrumented", steps=100), dict(real_boltz=1), text)
        self.assertTrue(result["atom_count_changed"])
        self.assertEqual(result["max_relative_error"], 1.0)
        final = next(row for row in result["samples"] if row["step"] == 100)
        self.assertEqual(final["kinetic_temperature_K"], 2.0)

    def test_truncated_nonfinite_or_missing_thermo_output_is_unassessed(self):
        for text in ("", "Step c_measured\n0 nan\n", "Step c_measured\n0\n"):
            with self.subTest(text=text), self.assertRaises(ValueError):
                replay.thermo_rows(text, ["Step", "c_measured"])
        with self.assertRaisesRegex(ValueError, "Missing instrumented"):
            replay.measurement(dict(id="gcmc-instrumented", steps=100), dict(real_boltz=1),
                               "Step Atoms KinEng c_NVT_temp c_Tfluid\n0 2 3 2 2\nLoop time\n")

    def test_reconstructed_correction_requires_a_unique_source_anchor(self):
        self.assertEqual(replay.replace_once("run 10\n", "run 10", "run 1"), "run 1\n")
        for text in ("run 1", "run 10\nrun 10"):
            with self.assertRaisesRegex(ValueError, "exactly one"):
                replay.replace_once(text, "run 10", "run 1")


if __name__ == "__main__":
    unittest.main()

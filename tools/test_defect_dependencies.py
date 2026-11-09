"""Exercise public recovery receipts and prevent unsupported source promotion."""

import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

import defect_dependencies as recovery
from corpus import write_json


ARCHIVE = recovery.ROOT / "docs/assets/defect-dependencies/2026-11-09"


class Response(io.BytesIO):
    code = 200
    url = "https://example.org/reference"

    class Headers:
        @staticmethod
        def get_content_type():
            return "text/plain"

    headers = Headers()


class DependencyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.snapshot = json.loads((ARCHIVE / "snapshot.json").read_bytes())
        self.assessment = json.loads(recovery.ASSESSMENT.read_bytes())
        self.snapshot_path = self.directory / "snapshot.json"
        self.assessment_path = self.directory / "assessment.json"

    def check(self, cache=None):
        write_json(self.snapshot_path, self.snapshot)
        self.assessment["snapshot_sha256"] = recovery.source.sha256(self.snapshot_path.read_bytes())
        write_json(self.assessment_path, self.assessment)
        return recovery.verify(self.snapshot_path, assessment_path=self.assessment_path, cache=cache)

    def test_frozen_recovery_report(self):
        result = recovery.verify(ARCHIVE / "snapshot.json")
        self.assertEqual(result, json.loads((ARCHIVE / "report.json").read_bytes()))
        self.assertEqual(result["summary"], {
            "cases": 2, "planned_urls": 6, "successful_http_responses": 5,
            "http_errors": 1, "unavailable_requests": 0, "current_reference_files": 3,
            "needs_evidence": 2, "new_replay_candidates": 0, "gold_defect_pairs": 0})

    def test_missing_duplicate_or_changed_sources(self):
        original = copy.deepcopy(self.snapshot)
        mutations = [lambda d: d.update(sources=[s for s in d["sources"] if s["id"] != "tempering-drive"]),
                     lambda d: d["sources"].append(copy.deepcopy(next(s for s in d["sources"] if s["id"] == "nist-entry"))),
                     lambda d: next(s for s in d["sources"] if s["id"] == "nist-entry").update(url="https://example.org"),
                     lambda d: d.update(plan_sha256="0" * 64)]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                self.snapshot = copy.deepcopy(original)
                mutate(self.snapshot)
                with self.assertRaises(ValueError):
                    self.check()

    def test_failures_are_retained_without_references(self):
        record = next(s for s in self.snapshot["sources"] if s["id"] == "tempering-drive")
        record.clear()
        record.update(id="tempering-drive", url=next(s["url"] for s in recovery.read_plan(recovery.PLAN)["sources"]
                                                   if s["id"] == "tempering-drive"),
                      outcome="unavailable", error="network unavailable")
        self.assertEqual(self.check()["summary"]["unavailable_requests"], 1)

    def test_html_error_page_cannot_be_a_reference_file(self):
        record = next(s for s in self.snapshot["sources"] if s["id"] == "nist-potential")
        record["content_type"] = "text/html"
        with self.assertRaises(ValueError):
            self.check()
        record["content_type"] = "text/plain"
        record["http_status"] = 404
        with self.assertRaises(ValueError):
            self.check()

    def test_cannot_promote_reference_or_claim_replays(self):
        original = copy.deepcopy(self.assessment)
        for field in ("native_replays", "checker_runs", "agent_trials", "gold_defect_pairs", "independent_adjudications"):
            with self.subTest(field=field):
                self.assessment = copy.deepcopy(original)
                self.assessment[field] = 1
                with self.assertRaises(ValueError):
                    self.check()
        self.assessment = copy.deepcopy(original)
        case = next(c for c in self.assessment["cases"] if c["id"] == "m67595")
        reference = next(r for r in case["references"] if r["source"] == "nist-potential")
        reference["relation"] = "reporter_original"
        with self.assertRaises(ValueError):
            self.check()
        reference["relation"] = "current_reference_not_original"
        case["decision"] = "replay_candidate"
        with self.assertRaises(ValueError):
            self.check()

    def test_cache_hash_and_literal_anchor_verification(self):
        for record in self.snapshot["sources"]:
            text = "fixture source bytes"
            for case in self.assessment["cases"]:
                for item in case["references"]:
                    if item["source"] == record["id"]:
                        text = "\n".join(item["anchors"])
            body = text.encode()
            (self.directory / record["id"]).write_bytes(body)
            record.update(bytes=len(body), sha256=recovery.source.sha256(body))
        self.assertEqual(self.check(self.directory)["summary"]["cases"], 2)
        case = next(c for c in self.assessment["cases"] if c["id"] == "m67595")
        next(r for r in case["references"] if r["source"] == "nist-potential")["anchors"] = ["invented parameter"]
        self.check()  # Offline metadata verification cannot verify uncached text.
        with self.assertRaisesRegex(ValueError, "anchor absent"):
            self.check(self.directory)
        (self.directory / "nist-potential").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "response differs"):
            self.check(self.directory)

    def test_fetch_records_http_and_network_errors_serially(self):
        calls = []

        def respond(request, timeout):
            self.assertEqual(timeout, 30)
            calls.append(request.full_url)
            if "drive.google" in request.full_url:
                raise urllib.error.HTTPError(request.full_url, 404, "Not Found", Response.headers,
                                             io.BytesIO(b"not found"))
            if request.full_url.endswith("README.md"):
                raise urllib.error.URLError("offline")
            return Response(b"reference")

        output = self.directory / "capture"
        with patch.object(recovery.urllib.request, "urlopen", side_effect=respond), patch("sys.stderr", io.StringIO()):
            result = recovery.fetch(recovery.PLAN, output)
        self.assertEqual(calls, [s["url"] for s in recovery.read_plan(recovery.PLAN)["sources"]])
        drive = next(s for s in result["sources"] if s["id"] == "tempering-drive")
        self.assertEqual(drive["http_status"], 404)
        self.assertEqual((output / "tempering-drive").read_bytes(), b"not found")
        self.assertEqual(next(s for s in result["sources"] if s["id"] == "meam-readme")["outcome"], "unavailable")
        with self.assertRaises(FileExistsError):
            recovery.fetch(recovery.PLAN, output)

    def test_cli_verifies_report_and_rejects_stale_report(self):
        command = [sys.executable, str(recovery.ROOT / "tools/defect_dependencies.py"), "verify",
                   "--snapshot", str(ARCHIVE / "snapshot.json"), "--report", str(ARCHIVE / "report.json")]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["summary"]["gold_defect_pairs"], 0)
        stale = self.directory / "report.json"
        stale.write_text("{}")
        result = subprocess.run(command[:-1] + [str(stale)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")

    def test_duplicate_references_and_missing_case_fail(self):
        case = next(c for c in self.assessment["cases"] if c["id"] == "m67595")
        original = copy.deepcopy(case["references"])
        case["references"].append(copy.deepcopy(next(r for r in original if r["source"] == "nist-potential")))
        with self.assertRaises(ValueError):
            self.check()
        case["references"] = original
        self.assessment["cases"] = [case]
        with self.assertRaises(ValueError):
            self.check()

    def test_fetch_bounds_and_cli_failure_exit(self):
        output = self.directory / "limited"
        with patch.object(recovery, "LIMIT", 4), patch.object(recovery.urllib.request, "urlopen",
                side_effect=lambda *args, **kwargs: Response(b"oversized")), patch("sys.stderr", io.StringIO()):
            result = recovery.fetch(recovery.PLAN, output)
        self.assertEqual(len(result["sources"]), 6)
        self.assertTrue(all(s["outcome"] == "unavailable" for s in result["sources"]))
        self.assertEqual({p.name for p in output.iterdir()}, {"snapshot.json"})
        with patch.object(sys, "argv", ["defect_dependencies.py", "fetch", "--output", str(self.directory / "cli")]), \
                patch.object(recovery, "fetch", return_value=result), patch("sys.stdout", io.StringIO()):
            self.assertEqual(recovery.main(), 1)


if __name__ == "__main__":
    unittest.main()

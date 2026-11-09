import contextlib
import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import defect_discovery as discovery


def topic(identity, pinned=False):
    return dict(id=identity, title=f"Discussion {identity}", slug=f"discussion-{identity}",
                pinned=pinned, category_id=40, posts_count=2)


class DiscoveryTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        plan = json.loads(discovery.PLAN.read_bytes())
        plan.update(capture_limit=2, previously_screened_topic_ids=[2])
        self.plan = self.root / "plan.json"
        self.plan.write_text(json.dumps(plan))
        self.pages = {0: [topic(1, pinned=True), topic(2), topic(3)],
                      1: [topic(3), topic(4)], 2: [topic(5)]}
        self.unavailable_pages = set()
        self.unavailable_topics = set()
        self.requests = []

    def read(self, url):
        self.requests.append(url)
        if "/c/lammps/40.json?page=" in url:
            page = int(url.rsplit("=", 1)[1])
            if page in self.unavailable_pages:
                raise OSError("Fixture page unavailable")
            return json.dumps(dict(topic_list=dict(topics=self.pages[page]))).encode()
        if "/posts.json?" in url:
            return json.dumps(dict(post_stream=dict(posts=[dict(id=101, post_number=2,
                username="responder", created_at="date", updated_at="date",
                cooked="<p>Unverified proposed correction.</p>")]))).encode()
        identity = int(url.rsplit("/", 1)[1].removesuffix(".json"))
        if identity in self.unavailable_topics:
            raise OSError("Fixture topic unavailable")
        first = dict(id=100, post_number=1, username="author", created_at="date",
                     updated_at="date", cooked="<p>Reported problem.</p><pre>run 0</pre>")
        return json.dumps(dict(id=identity, title=f"Discussion {identity}",
                               post_stream=dict(posts=[first], stream=[100, 101]))).encode()

    def fetch(self):
        with patch.object(discovery.source, "read_url", side_effect=self.read), \
                contextlib.redirect_stderr(io.StringIO()):
            report = discovery.fetch(self.plan, self.root / "cache")
        return report

    def publish(self):
        self.fetch()
        at = self.root / "public"
        discovery.publish(self.root / "cache", at)
        return at

    def test_listing_order_deduplicates_without_dropping_excluded_or_deferred_topics(self):
        report = self.fetch()
        self.assertEqual(report["summary"], dict(requested_pages=3, collected_pages=3,
            listing_slots=6, unique_topics=5, duplicate_slots=1, selected_captures=2,
            outcomes=dict(budget_deferred=1, captured=2, pinned=1, previously_screened=1)))
        by_id = {candidate["id"]: candidate for candidate in report["candidates"]}
        self.assertEqual(by_id["m3"]["occurrences"], [dict(page=0, row=2), dict(page=1, row=0)])
        self.assertEqual(by_id["m3"]["structural"], dict(posts=2, initial_post_code_blocks=1,
            all_post_code_blocks=1, downloaded_attachments=0, unavailable_attachments=0))
        self.assertFalse(any("discussion-5/5.json" in request for request in self.requests))
        self.assertEqual(report["screened_topics"], 0)
        self.assertEqual(report["gold_defect_pairs"], 0)
        self.assertFalse(report["checker_evaluated"])
        form = discovery.screening(report)
        self.assertEqual({case["id"] for case in form["cases"]}, {"m3", "m4"})
        self.assertTrue(all(case["decision"] == "pending" and case["condition"] is None
                            for case in form["cases"]))
        self.assertNotIn("LML-", discovery.packet(report))
        budget = json.loads((self.root / "cache/budget.json").read_bytes())
        self.assertEqual(budget["listing"]["remaining_pages"], 2)
        self.assertEqual(budget["thread"]["remaining_topics"], 1)

    def test_capture_failure_is_not_replaced_and_remains_in_screening_queue(self):
        self.unavailable_topics.add(3)
        report = self.fetch()
        by_id = {candidate["id"]: candidate for candidate in report["candidates"]}
        self.assertEqual(by_id["m3"]["outcome"], "capture_failed")
        self.assertEqual(by_id["m4"]["outcome"], "captured")
        self.assertEqual(by_id["m5"]["outcome"], "budget_deferred")
        self.assertEqual(report["summary"]["selected_captures"], 2)
        case = next(case for case in discovery.screening(report)["cases"] if case["id"] == "m3")
        self.assertEqual(case["capture_outcome"], "capture_failed")
        self.assertEqual(case["decision"], "pending")

    def test_unavailable_page_is_retained_and_all_later_planned_pages_are_requested(self):
        self.unavailable_pages.add(1)
        report = self.fetch()
        self.assertEqual(report["summary"]["requested_pages"], 3)
        self.assertEqual(report["summary"]["collected_pages"], 2)
        self.assertEqual(report["summary"]["listing_slots"], 4)
        self.assertIn("https://matsci.org/c/lammps/40.json?page=2", self.requests)
        pages = json.loads((self.root / "cache/listing.json").read_bytes())
        failed = next(page for page in pages if page["page"] == 1)
        self.assertEqual(failed["status"], "unavailable")
        self.assertEqual(failed["topics"], [])
        discovery.verify(self.root / "cache", self.root / "cache")

    def test_public_archive_is_deterministic_verifiable_and_contains_no_raw_threads(self):
        at = self.publish()
        self.assertEqual({p.name for p in at.iterdir()}, set(discovery.PUBLIC) | {"capture.json", "inventory.json"})
        command = [sys.executable, str(discovery.ROOT / "tools/defect_discovery.py"),
                   "verify", "--archive", str(at)]
        first = subprocess.run(command, capture_output=True, check=True)
        second = subprocess.run(command, capture_output=True, check=True)
        self.assertEqual(first.stdout, second.stdout)
        self.assertEqual(json.loads(first.stdout), dict(unique_topics=5, selected_captures=2,
            cached_bytes_verified=False, screened_topics=0, gold_defect_pairs=0, checker_evaluated=False))
        self.assertTrue(discovery.verify(at, self.root / "cache")["cached_bytes_verified"])
        with self.assertRaises(FileExistsError):
            discovery.fetch(self.plan, self.root / "cache")

    def test_changed_source_listing_and_post_receipts_fail_cached_verification(self):
        at = self.publish()
        page = self.root / "cache/page-0.json"
        original = page.read_bytes()
        page.write_bytes(original + b" ")
        with self.assertRaisesRegex(ValueError, "listing bytes"):
            discovery.verify(at, self.root / "cache")
        page.write_bytes(original)
        path = self.root / "cache/threads/m3/topic.json"
        path.write_bytes(path.read_bytes() + b" ")
        with self.assertRaisesRegex(ValueError, "source bytes"):
            discovery.verify(at, self.root / "cache")

    def test_changed_or_missing_capture_outcomes_and_false_labels_are_rejected(self):
        at = self.publish()
        report = json.loads((at / "report.json").read_bytes())
        for mutation in ("gold", "screened", "missing", "duplicate", "false-summary"):
            changed = copy.deepcopy(report)
            if mutation == "gold":
                changed["gold_defect_pairs"] = 1
            elif mutation == "screened":
                changed["screened_topics"] = 2
            elif mutation == "missing":
                changed["candidates"] = [c for c in changed["candidates"] if c["id"] != "m5"]
            elif mutation == "duplicate":
                changed["candidates"].append(next(c for c in changed["candidates"] if c["id"] == "m3"))
            else:
                changed["summary"]["unique_topics"] = 900
            (at / "report.json").write_text(json.dumps(changed))
            inventory = json.loads((at / "inventory.json").read_bytes())
            inventory["report.json"] = discovery.source.sha256((at / "report.json").read_bytes())
            (at / "inventory.json").write_text(json.dumps(inventory))
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, "report or pending"):
                discovery.verify(at)

    def test_missing_page_and_unsafe_listing_identity_are_rejected(self):
        data = {"topic_list": {"topics": [dict(topic(3), slug="../../other")]}}
        with self.assertRaisesRegex(ValueError, "identity"):
            discovery.parse_page(json.dumps(data).encode())
        plan = discovery.read_plan(self.plan)
        with self.assertRaisesRegex(ValueError, "Every requested page"):
            discovery.select(plan, [dict(page=0, status="collected", topics=[])])

    def test_malformed_page_is_retained_with_raw_receipt(self):
        original = self.read

        def read(url):
            return b"not JSON" if url.endswith("page=1") else original(url)

        with patch.object(discovery.source, "read_url", side_effect=read), \
                contextlib.redirect_stderr(io.StringIO()):
            report = discovery.fetch(self.plan, self.root / "cache")
        self.assertEqual(report["summary"]["collected_pages"], 2)
        page = next(p for p in json.loads((self.root / "cache/listing.json").read_bytes()) if p["page"] == 1)
        self.assertEqual(page["status"], "unavailable")
        self.assertEqual(page["raw"]["sha256"], discovery.source.sha256(b"not JSON"))
        discovery.verify(self.root / "cache", self.root / "cache")

    def test_duplicate_capture_and_dropped_failure_are_rejected(self):
        self.unavailable_topics.add(3)
        self.fetch()
        pages = json.loads((self.root / "cache/listing.json").read_bytes())
        snapshot = json.loads((self.root / "cache/sources.json").read_bytes())
        for mutation in ("duplicate", "drop"):
            changed = copy.deepcopy(snapshot)
            if mutation == "duplicate":
                changed["sources"].append(next(s for s in changed["sources"] if s["id"] == "m4"))
            else:
                changed["failures"].clear()
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, "Every selected topic"):
                discovery.assemble(self.plan, pages, changed)

    def test_attachment_presence_does_not_admit_a_defect_and_unavailable_links_remain(self):
        original = self.read

        def read(url):
            if url.endswith("/uploads/input.in"):
                return b"run 0\n"
            if url.endswith("/uploads/missing.data"):
                raise OSError("Fixture attachment unavailable")
            data = original(url)
            if url.endswith("discussion-3/3.json"):
                value = json.loads(data)
                post = next(p for p in value["post_stream"]["posts"] if p["post_number"] == 1)
                post["cooked"] += ('<a class="attachment" href="/uploads/input.in">input</a>' * 2
                                   + '<a class="attachment" href="/uploads/missing.data">missing</a>')
                data = json.dumps(value).encode()
            return data

        with patch.object(discovery.source, "read_url", side_effect=read), \
                contextlib.redirect_stderr(io.StringIO()):
            report = discovery.fetch(self.plan, self.root / "cache")
        candidate = next(c for c in report["candidates"] if c["id"] == "m3")
        self.assertEqual(candidate["outcome"], "captured")
        self.assertEqual(candidate["structural"]["downloaded_attachments"], 1)
        self.assertEqual(candidate["structural"]["unavailable_attachments"], 1)
        self.assertEqual(report["gold_defect_pairs"], 0)
        self.assertEqual(report["screened_topics"], 0)
        discovery.publish(self.root / "cache", self.root / "public")

    def test_cli_failure_exit_preserves_capture_outcomes(self):
        self.unavailable_topics.add(3)
        stdout = io.StringIO()
        with patch.object(sys, "argv", ["defect_discovery.py", "fetch", "--plan", str(self.plan),
                                        "--output", str(self.root / "cache")]), \
                patch.object(discovery.source, "read_url", side_effect=self.read), \
                contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(discovery.main(), 1)
        self.assertEqual(json.loads(stdout.getvalue())["outcomes"]["capture_failed"], 1)
        self.assertTrue((self.root / "cache/screening.json").is_file())

    def test_public_verification_requires_its_complete_inventory(self):
        at = self.publish()
        (at / "inventory.json").unlink()
        with self.assertRaisesRegex(ValueError, "Missing public"):
            discovery.verify(at)


if __name__ == "__main__":
    unittest.main()

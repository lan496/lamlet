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

import defect_screening as screening


REPORT = screening.ROOT / "docs/assets/defect-screening/2026-11-09/report.json"


class ScreeningTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.data = json.loads(screening.MANIFEST.read_bytes())
        self.manifest = self.root / "screening.json"

    def validate(self, data=None):
        self.manifest.write_text(json.dumps(data if data is not None else self.data))
        return screening.validate(self.manifest)

    def test_complete_published_cohort_has_only_model_assisted_decisions(self):
        result = screening.validate()
        self.assertEqual(result["summary"], dict(selected_topics=12, screened_topics=12,
            covered_posts=67, decisions=dict(excluded=10, needs_evidence=2, replay_candidate=0),
            classifications=dict(announcement=2, build_environment=3, engine_report=2,
                                 feature_request=1, input_report=2, usage_question=2)))
        self.assertEqual(result, json.loads(REPORT.read_bytes()))
        self.assertFalse(result["review"]["independent"])
        self.assertEqual(result["gold_defect_pairs"], 0)
        archive = screening.ROOT / "docs/assets/defect-discovery/2026-11-09/screening.json"
        self.assertEqual({case["decision"] for case in json.loads(archive.read_bytes())["cases"]}, {"pending"})
        self.assertEqual(next(c for c in self.data["cases"] if c["id"] == "m67595")["read_post_numbers"],
                         [1, 2, 3, 4, 5, 6, 8, 9])

    def test_cli_verification_is_deterministic_and_discloses_uncached_anchors(self):
        command = [sys.executable, str(screening.ROOT / "tools/defect_screening.py"),
                   "verify", "--report", str(REPORT)]
        first = subprocess.run(command, check=True, capture_output=True)
        second = subprocess.run(command, check=True, capture_output=True)
        self.assertEqual(first.stdout, second.stdout)
        self.assertEqual(json.loads(first.stdout), dict(screened_topics=12,
            cached_anchors_verified=False, independently_adjudicated=0, gold_defect_pairs=0))
        stale = self.root / "report.json"
        report = json.loads(REPORT.read_bytes())
        report["summary"]["screened_topics"] = 11
        stale.write_text(json.dumps(report))
        failed = subprocess.run(command[:-1] + [str(stale)], capture_output=True)
        self.assertEqual(failed.returncode, 2)
        self.assertEqual(failed.stdout, b"")
        self.assertIn("Published screening report differs", json.loads(failed.stderr)["error"])

    def test_missing_and_duplicate_topic_decisions_are_rejected(self):
        for mutation in ("missing", "duplicate"):
            data = copy.deepcopy(self.data)
            subject = next(c for c in data["cases"] if c["id"] == "m67595")
            if mutation == "missing":
                data["cases"].remove(subject)
            else:
                subject["id"] = "m67540"
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, "Every selected topic"):
                self.validate(data)

    def test_missing_invented_and_duplicate_post_coverage_are_rejected(self):
        for mutation in ("missing", "invented", "duplicate", "bool"):
            data = copy.deepcopy(self.data)
            numbers = next(c for c in data["cases"] if c["id"] == "m67595")["read_post_numbers"]
            if mutation == "missing":
                numbers.remove(9)
            elif mutation == "invented":
                numbers.append(7)
                numbers.sort()
            elif mutation == "bool":
                numbers.remove(1)
                numbers.insert(0, True)
            else:
                numbers.append(9)
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, "post coverage"):
                self.validate(data)

    def test_evidence_requires_existing_posts_and_the_final_reply(self):
        for mutation in ("final", "invented", "empty", "long"):
            data = copy.deepcopy(self.data)
            case = next(c for c in data["cases"] if c["id"] == "m67595")
            if mutation == "final":
                case["evidence"] = [e for e in case["evidence"] if e["post_number"] != 9]
            else:
                item = next(e for e in case["evidence"] if e["post_number"] == 1)
                if mutation == "invented":
                    item["post_number"] = 7
                else:
                    item["anchor"] = "" if mutation == "empty" else "word " * 13
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.validate(data)

    def test_scientific_and_independent_label_promotions_are_rejected(self):
        for field in ("gold_defect_pairs", "independently_adjudicated", "native_replays", "agent_trials",
                      "review", "split", "checker_evaluated", "plan_sha256"):
            data = copy.deepcopy(self.data)
            data[field] = {"kind": "human", "independent": True} if field == "review" else (
                "held_out" if field == "split" else "stale" if field == "plan_sha256" else 1)
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "provenance or review"):
                self.validate(data)

    def test_evidence_gaps_and_input_provenance_prevent_replay_promotion(self):
        for mutation in ("gaps", "fragment", "classification", "exclusion", "missing-gap"):
            data = copy.deepcopy(self.data)
            case = next(c for c in data["cases"] if c["id"] == "m67595")
            if mutation in {"gaps", "fragment"}:
                case["decision"] = "replay_candidate"
                if mutation == "fragment":
                    case["dependency_gaps"] = []
            elif mutation == "classification":
                case["classification"] = "engine_report"
            elif mutation == "exclusion":
                case["decision"] = "excluded"
            else:
                case["dependency_gaps"] = []
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.validate(data)

    def test_all_attachments_and_inspection_receipts_are_required(self):
        for mutation in ("missing", "duplicate", "hash", "fake-read"):
            data = copy.deepcopy(self.data)
            case = next(c for c in data["cases"] if c["id"] == "m67540")
            artifact = next(a for a in case["artifacts"] if a["file"] == "attachment-95129ee2260e4c4a")
            if mutation == "missing":
                case["artifacts"].remove(artifact)
            elif mutation == "duplicate":
                case["artifacts"].append(artifact)
            elif mutation == "hash":
                artifact["sha256"] = "0" * 64
            else:
                artifact["inspection"] = "receipt_only"
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.validate(data)

    def fixture(self):
        discovery = screening.discovery
        plan = json.loads(discovery.PLAN.read_bytes())
        plan.update(pages=[0], capture_limit=1, previously_screened_topic_ids=[])
        source_plan = self.root / "discovery-plan.json"
        source_plan.write_text(json.dumps(plan))
        topic = dict(id=123, title="Fixture", slug="fixture", pinned=False, category_id=40, posts_count=2)
        posts = [dict(id=100, post_number=1, username="author", created_at="date", updated_at="date",
                      cooked='<p>Original report.</p><pre>run 0</pre><a class="attachment" href="/uploads/input.in">Input</a>'),
                 dict(id=101, post_number=2, username="reply", created_at="date", updated_at="date",
                      cooked="<p>Correction remains unverified.</p>")]

        def read(url):
            if url.endswith("input.in"):
                return b"write_restart restart\n"
            value = dict(topic_list=dict(topics=[topic])) if "page=" in url else dict(
                id=123, title="Fixture", post_stream=dict(posts=posts, stream=[100, 101]))
            return json.dumps(value).encode()

        cache, archive = self.root / "cache", self.root / "archive"
        with patch.object(discovery.source, "read_url", side_effect=read), contextlib.redirect_stderr(io.StringIO()):
            discovery.fetch(source_plan, cache)
        discovery.publish(cache, archive)
        plan = json.loads(screening.PLAN.read_bytes())
        plan.update(discovery_archive="archive", discovery_inventory_sha256=discovery.source.sha256(
            (archive / "inventory.json").read_bytes()))
        plan_path = self.root / "screening-plan.json"
        plan_path.write_text(json.dumps(plan))
        data = copy.deepcopy(self.data)
        case = next(c for c in data["cases"] if c["id"] == "m67540")
        source = next(s for s in json.loads((archive / "sources.json").read_bytes())["sources"] if s["id"] == "m123")
        attachment = next(a for a in source["attachments"] if a["status"] == "downloaded")
        case.update(id="m123", url="https://matsci.org/t/fixture/123", read_post_numbers=[1, 2],
                    evidence=[dict(post_number=1, anchor="Original report."),
                              dict(post_number=2, anchor="Correction remains unverified.")],
                    artifacts=[dict(file=attachment["file"], sha256=attachment["sha256"], status="downloaded",
                                    inspection="text_read", role="Input fragment", anchors=["write_restart restart"])])
        data.update(cases=[case], plan_sha256=discovery.source.sha256(plan_path.read_bytes()))
        self.manifest.write_text(json.dumps(data))
        return plan_path, cache, data

    def test_exact_cache_checks_post_and_attachment_anchors_without_execution(self):
        plan, cache, data = self.fixture()
        with patch.object(screening, "ROOT", self.root), patch.object(
                screening.discovery.source, "read_url", side_effect=AssertionError("No network")):
            self.assertEqual(screening.validate(self.manifest, plan, cache)["summary"]["covered_posts"], 2)
            subject = next(c for c in data["cases"] if c["id"] == "m123")
            item = next(e for e in subject["evidence"] if e["post_number"] == 2)
            item["anchor"] = "Invented correction."
            self.manifest.write_text(json.dumps(data))
            # Offline metadata verification cannot check uncached text.
            screening.validate(self.manifest, plan)
            with self.assertRaisesRegex(ValueError, "captured post"):
                screening.validate(self.manifest, plan, cache)
            item["anchor"] = "Correction remains unverified."
            artifact = next(a for a in subject["artifacts"] if a["inspection"] == "text_read")
            artifact["anchors"] = ["read_restart other"]
            self.manifest.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "captured attachment"):
                screening.validate(self.manifest, plan, cache)

    def test_stale_source_bytes_and_inventory_are_rejected(self):
        plan, cache, data = self.fixture()
        with patch.object(screening, "ROOT", self.root):
            file = cache / "threads/m123/topic.json"
            file.write_bytes(file.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "source bytes"):
                screening.validate(self.manifest, plan, cache)
            inventory = self.root / "archive/inventory.json"
            inventory.write_bytes(inventory.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "discovery inventory"):
                screening.validate(self.manifest, plan)

    def test_replay_preparation_remains_separate_from_gold_admission(self):
        plan, cache, data = self.fixture()
        subject = next(c for c in data["cases"] if c["id"] == "m123")
        subject.update(decision="replay_candidate", dependency_gaps=[],
                       proposed_correction="Use a distinct filename per replica.")
        self.manifest.write_text(json.dumps(data))
        with patch.object(screening, "ROOT", self.root):
            report = screening.validate(self.manifest, plan, cache)
            self.assertEqual(report["summary"]["decisions"]["replay_candidate"], 1)
            self.assertEqual(report["gold_defect_pairs"], 0)
            subject["proposed_correction"] = None
            self.manifest.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "Replay preparation"):
                screening.validate(self.manifest, plan, cache)


if __name__ == "__main__":
    unittest.main()

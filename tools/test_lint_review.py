import copy
import json
from pathlib import Path
import tempfile
import unittest

import lint_review as review


class ReviewTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name) / "review"
        review.prepare(review.CORPUS, self.output)
        self.template = json.loads((self.output / "review.json").read_text())

    def test_packet_has_every_input_references_and_reconstructed_difference(self):
        packet = (self.output / "packet.md").read_text()
        self.assertEqual(len(self.template["cases"]), 45)
        for case in self.template["cases"]:
            self.assertIn("## " + case["id"] + "\n", packet)
        self.assertIn("+fix sample all ave/time 2 6 10 c_thermal", packet)
        self.assertIn("doc/src/fix_ave_time.rst", packet)
        report = review.validate(review.CORPUS, self.template)
        self.assertEqual(report["decisions"], {"pending": 45})
        self.assertFalse(report["review_complete"])
        self.assertFalse(report["all_labels_agreed"])

    def test_published_packet_is_current_and_does_not_claim_review(self):
        published = review.CORPUS.parents[1].parent / "docs/assets/lint-review"
        for name in ("packet.md", "review.json"):
            self.assertEqual((published / name).read_bytes(), (self.output / name).read_bytes())

    def test_reasons_attestation_and_disagreements_remain_distinct(self):
        for case in self.template["cases"]:
            case.update(decision="agree", reason="Reviewed the cited condition.")
        self.assertFalse(review.validate(review.CORPUS, self.template)["review_complete"])
        self.template["reviewer"] = dict(name="Test reviewer", affiliation="Test only",
            date="2026-11-09", independent_of_implementation_and_labels=True)
        self.assertTrue(review.validate(review.CORPUS, self.template)["all_labels_agreed"])
        case = next(c for c in self.template["cases"] if c["id"] == "averaging-boundary")
        case["decision"] = "disagree"
        result = review.validate(review.CORPUS, self.template)
        self.assertTrue(result["review_complete"])
        self.assertFalse(result["all_labels_agreed"])
        self.assertFalse(result["identity_verified_by_tool"])
        case["reason"] = ""
        with self.assertRaisesRegex(ValueError, "requires a reason"):
            review.validate(review.CORPUS, self.template)

    def test_missing_duplicate_stale_or_relabelled_reviews_fail(self):
        for mutation in ("missing", "duplicate", "stale", "corpus", "invalid"):
            changed = copy.deepcopy(self.template)
            entry = next(c for c in changed["cases"] if c["id"] == "averaging-boundary")
            if mutation == "missing":
                changed["cases"].remove(entry)
            elif mutation == "duplicate":
                changed["cases"].append(entry)
            elif mutation == "stale":
                entry["input_sha256"] = "stale"
            elif mutation == "corpus":
                changed["corpus_sha256"] = "stale"
            else:
                entry["decision"] = "approved"
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                review.validate(review.CORPUS, changed)


if __name__ == "__main__":
    unittest.main()

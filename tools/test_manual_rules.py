import json
import unittest

from lammps_docs import ROOT, REVISION
from manual_rules import locate, reference, review_ledger, selectors


class ManualRuleTests(unittest.TestCase):
    def test_wrapped_citations_preserve_actual_source_offsets(self):
        text = "Heading\n=======\n\nThis command must come after the\nsimulation box is defined.\n"
        start, end = locate(text, "This command must come after the simulation box is defined")
        self.assertEqual(text[start:end], "This command must come after the\nsimulation box is defined")
        self.assertEqual(text.count("\n", 0, start) + 1, 4)
        with self.assertRaises(ValueError):
            locate(text, "This command must come before the simulation box is defined")

    def test_style_selectors_do_not_confuse_id_group_and_style_positions(self):
        self.assertEqual(selectors({"commands": ["fix nvt", "compute msd", "pair_style dpd", "units"]}),
                         [{"command": "fix", "position": 3, "value": "nvt"},
                          {"command": "compute", "position": 3, "value": "msd"},
                          {"command": "pair_style", "position": 1, "value": "dpd"},
                          {"command": "units"}])

    def test_every_executable_rule_has_a_hash_bound_official_citation(self):
        catalog = json.loads((ROOT / "crates/lamlet/data/lammps_docs.json").read_text())
        rules = json.loads((ROOT / "crates/lamlet/data/manual_rules.json").read_text())
        pages = {p["path"]: p for p in catalog["pages"]}
        self.assertEqual(rules["revision"], REVISION)
        self.assertEqual(len({r["id"] for r in rules["rules"]}), len(rules["rules"]))
        for rule in rules["rules"]:
            with self.subTest(rule=rule["id"]):
                source = rule["source"]
                self.assertEqual(source["sha256"], pages[source["page"]]["sha256"])
                self.assertEqual(source["url"], pages[source["page"]]["source_url"] + "#L" + str(source["start_line"]))
                self.assertGreaterEqual(source["end_line"], source["start_line"])
                self.assertFalse(rule["autofix"])
                self.assertEqual(rule["severity"], "warning")

    def test_distinct_drude_seed_positions_and_ttm_variant_grammars(self):
        rules = json.loads((ROOT / "crates/lamlet/data/manual_rules.json").read_text())["rules"]
        by_id = {r["id"]: r for r in rules}
        self.assertEqual(by_id["LML-MAN-FIX-LANGEVIN-DRUDE-SEED_COM"]["predicate"]["position"], 6)
        self.assertEqual(by_id["LML-MAN-FIX-LANGEVIN-DRUDE-SEED_DRUDE"]["predicate"]["position"], 9)
        self.assertEqual(by_id["LML-MAN-FIX-TTM-SEED"]["selectors"], [{"command": "fix", "position": 3, "value": "ttm"}])

    def test_generated_reference_matches_the_executable_catalog(self):
        manifest = json.loads((ROOT / "crates/lamlet/data/manual_rules.json").read_text())
        self.assertEqual((ROOT / "docs/manual-rule-catalog.md").read_text(), reference(manifest))
        self.assertFalse(manifest["review"]["complete"])
        self.assertGreater(manifest["review"]["pending_paragraphs"], 0)

    def test_rule_links_do_not_silently_mark_paragraphs_reviewed(self):
        texts = {"test.rst": "This command must come after the simulation box is defined.\n"}
        rules = [{"id": "LML-MAN-TEST", "source": {"page": "test.rst", "start_line": 1, "end_line": 1}}]
        reviews = {"revision": REVISION, "paragraphs": []}
        ledger, summary = review_ledger(texts, rules, reviews)
        self.assertEqual(ledger[0]["related_rules"], ["LML-MAN-TEST"])
        self.assertEqual(ledger[0]["status"], "pending")
        self.assertFalse(summary["complete"])
        review = {"page": "test.rst", "sha256": ledger[0]["sha256"], "rules": ["LML-MAN-TEST"],
                  "status": "implemented", "reason": "Known box state is checked."}
        reviews["paragraphs"] = [review]
        self.assertTrue(review_ledger(texts, rules, reviews)[1]["complete"])
        reviews["paragraphs"] = [review | {"status": "partial"}]
        self.assertFalse(review_ledger(texts, rules, reviews)[1]["complete"])
        with self.assertRaisesRegex(ValueError, "Stale review"):
            review_ledger({"test.rst": "A changed requirement must be reviewed.\n"}, rules, reviews)
        reviews["paragraphs"] = [review | {"rules": ["LML-MAN-NONEXISTENT"]}]
        with self.assertRaisesRegex(ValueError, "unknown rule"):
            review_ledger(texts, rules, reviews)


if __name__ == "__main__":
    unittest.main()

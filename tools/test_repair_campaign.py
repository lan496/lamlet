import json
from pathlib import Path
import tempfile
import unittest

import repair_campaign as campaign


class RepairCampaignTests(unittest.TestCase):
    def test_assignment_is_reproducible_complete_and_excludes_reserve(self):
        assignments = campaign.order(496)
        self.assertEqual(assignments, campaign.order(496))
        self.assertEqual(len(assignments), 6)
        self.assertEqual({(t["task"], t["condition"]) for t in assignments},
            {(task, condition) for task in ("integrator-dev", "orientation-dev", "sampling-dev")
             for condition in ("A", "C")})
        self.assertNotEqual(assignments, campaign.order(497))

    def test_missing_results_are_unassessed_and_frozen_edits_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            trial = {"id": "integrator-dev-A", "task": "integrator-dev", "condition": "A"}
            self.assertEqual(campaign.measure(trial, root)["status"], "unassessed")
            path = root / "frozen.in"
            path.write_text("run 20\n")
            files = {"frozen.in": campaign.digest(path)}
            campaign.frozen(root, files)
            path.write_text("run 0\n")
            with self.assertRaisesRegex(ValueError, "Frozen file changed"):
                campaign.frozen(root, files)

    def test_success_requires_both_independent_grade_and_transport_reward(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            trial = {"id": "integrator-dev-C", "task": "integrator-dev", "condition": "C"}
            directory = root / "integrator-dev__test"
            (directory / "verifier").mkdir(parents=True)
            (directory / "verifier/assessment.json").write_text('{"status":"correct"}')
            for reward, expected in ((0, False), (1, True)):
                (directory / "result.json").write_text(json.dumps({
                    "verifier_result": {"rewards": {"reward": reward}},
                    "agent_execution": {"started_at": "2026-11-09T00:00:00Z",
                                        "finished_at": "2026-11-09T00:00:12Z"}}))
                result = campaign.measure(trial, root)
                self.assertEqual(result["success"], expected)
                self.assertEqual(result["agent_seconds"], 12)


if __name__ == "__main__":
    unittest.main()

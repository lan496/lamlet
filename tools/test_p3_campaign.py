import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

from p3_campaign import assess, publish, trial_order


class CampaignTest(unittest.TestCase):
    def test_schedule_has_one_trial_per_condition_per_block(self):
        trials = trial_order(496)
        self.assertEqual({(r["condition"], r["block"]) for r in trials},
                         {(c, b) for c in ("A", "C") for b in (1, 2, 3)})
        self.assertEqual(len(trials), 6)

    def test_runner_reward_cannot_replace_independent_assessment(self):
        with tempfile.TemporaryDirectory() as temporary:
            job = Path(temporary) / "jobs/C1"
            folder = job / "nb-impact__trial"
            folder.mkdir(parents=True)
            data = {"verifier_result": {"rewards": {"reward": 1}},
                    "exception_info": None}
            (folder / "result.json").write_text(json.dumps(data))
            trial = {"id": "C1", "condition": "C", "block": 1}
            self.assertFalse(assess(job, trial)["success"])
            verifier = folder / "verifier"
            verifier.mkdir()
            assessment = verifier / "assessment.json"
            assessment.write_text(json.dumps({"scientific_status": "correct"}))
            self.assertTrue(assess(job, trial)["success"])
            data["exception_info"] = {"exception_type": "AgentTimeoutError"}
            (folder / "result.json").write_text(json.dumps(data))
            self.assertFalse(assess(job, trial)["success"])

    def test_public_archive_preserves_unknowns_and_excludes_model_reasoning(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            campaign, public = root / "campaign", root / "public"
            campaign.mkdir()
            trials = []
            for trial in trial_order(496):
                folder = campaign / "jobs" / trial["id"] / "nb-impact__trial"
                (folder / "agent").mkdir(parents=True)
                trajectory = {"steps": [{"step_id": 1, "message": "PRIVATE_REASONING",
                                          "tool_calls": [{"function": {"arguments": {
                                              "cmd": "lamlet check solution.in --format=json"}}}],
                                          "observation": "Unknown command"}]}
                (folder / "agent/trajectory.json").write_text(json.dumps(trajectory))
                trials.append({**trial, "scientific_status": "unassessed", "success": False,
                               "reward": None, "agent_seconds": 10, "verifier_seconds": 2,
                               "exception": None, "assessment": {"scientific_status": "unassessed"}})
            for condition in ("A", "C"):
                task = campaign / "tasks" / condition / "nb-impact"
                task.mkdir(parents=True)
                (task / "instruction.md").write_text("Frozen task")
            (campaign / "results.json").write_text(json.dumps({
                "complete": True, "manifest": {}, "trials": trials}))
            report = root / "report.json"
            report.write_text("{}")
            publish(SimpleNamespace(output=campaign, public=public, replay=report,
                                    admission=root, harbor_admission=report, static_report=report))
            summary = json.loads((public / "pilot.json").read_text())
            self.assertEqual(summary["conditions"]["C"]["success_bounds"], [0, 1])
            self.assertEqual(summary["conditions"]["A"]["unassessed"], 3)
            self.assertTrue(all(t["lamlet_check_invoked"] for t in summary["trials"]))
            self.assertNotIn("PRIVATE_REASONING", "".join(p.read_text() for p in public.rglob("*.json")))


if __name__ == "__main__":
    unittest.main()

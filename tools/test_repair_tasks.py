from pathlib import Path
import unittest

import repair_tasks as repair


class RepairTaskTests(unittest.TestCase):
    def test_development_contracts_accept_references_and_reject_original_defects(self):
        for identity, task in repair.tasks().items():
            if task["split"] != "development":
                continue
            with self.subTest(task=identity):
                self.assertTrue(repair.contract(task, task["reference"])[0])
                self.assertFalse(repair.contract(task, task["input"])[0])

    def test_counterexamples_do_not_erase_the_task_or_change_the_physics(self):
        task = repair.tasks()["integrator-dev"]
        for candidate in (task["reference"].replace("run 40", "run 0"),
                          task["reference"].replace("unfix thermal", "unfix ballistic"),
                          task["reference"].replace("fix ballistic all nve", "fix ballistic all nvt temp 0.85 0.85 0.3")):
            self.assertFalse(repair.contract(task, candidate)[0])
        task = repair.tasks()["sampling-dev"]
        self.assertFalse(repair.contract(task, task["reference"].replace("4 7 28", "2 7 14"))[0])
        task = repair.tasks()["orientation-dev"]
        self.assertFalse(repair.contract(task, task["reference"].replace("orient y -1 1 0", "orient y 1 -1 0"))[0])
        self.assertTrue(repair.contract(task, task["reference"].replace("orient y -1 1 0", "orient y -2 2 0"))[0])
        self.assertTrue(repair.contract(task, "# comment\n" + task["reference"])[0])

    def test_unsupported_syntax_and_missing_engine_remain_unassessed(self):
        task = repair.tasks()["integrator-dev"]
        self.assertEqual(repair.assess(task, task["reference"] + "run ${steps}\n", Path("missing"))["status"], "unassessed")
        self.assertEqual(repair.assess(task, task["reference"], Path("missing"))["status"], "unassessed")


if __name__ == "__main__":
    unittest.main()

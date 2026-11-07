"""Test task packaging and reward transport, without claiming physics validation."""

import argparse
from contextlib import redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import harbor_tasks
import harbor_verifier


class HarborTests(unittest.TestCase):
    def payload(self):
        return {"schema_version": 1, "task": "P3", "submission_sha256": "script-hash",
                "oracle_version": "fixture-only", "criteria": {
                    "gap": {"status": "pass", "evidence": ["fixture observation"]},
                    "velocity": {"status": "pass", "evidence": ["fixture observation"]},
                }}

    def test_only_complete_evidence_can_produce_a_success_reward(self):
        task = {"id": "P3", "criteria": ["gap", "velocity"]}
        payload = self.payload()
        self.assertEqual(harbor_verifier.assess(payload, task, "script-hash"), ("correct", 1))
        payload["criteria"]["gap"]["status"] = "unknown"
        self.assertEqual(harbor_verifier.assess(payload, task, "script-hash"), ("unassessed", None))
        payload["criteria"]["velocity"]["status"] = "fail"
        self.assertEqual(harbor_verifier.assess(payload, task, "script-hash"), ("incorrect", 0))

    def test_missing_criteria_or_evidence_and_wrong_identity_are_not_success(self):
        task = {"id": "P3", "criteria": ["gap", "velocity"]}
        mutations = [
            lambda p: p.update(submission_sha256="another-script"),
            lambda p: p.update(task="P1"),
            lambda p: p["criteria"].pop("gap"),
            lambda p: p["criteria"]["gap"].update(evidence=[]),
            lambda p: p["criteria"]["gap"].update(status="probably"),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                payload = self.payload()
                mutate(payload)
                with self.assertRaises(ValueError):
                    harbor_verifier.assess(payload, task, "script-hash")
        with self.assertRaises(ValueError):
            harbor_verifier.assess({**self.payload(), "criteria": {}},
                                   {"id": "P3", "criteria": []}, "script-hash")

    def test_missing_oracle_removes_stale_rewards_without_using_agent_claims(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            task = root / "task.json"
            task.write_text(json.dumps({"id": "P3", "criteria": ["gap"]}))
            submission = root / "solution.in"
            submission.write_text("run 0\n")
            (root / "agent-claims.json").write_text('{"scientifically_correct":true}')
            logs = root / "logs"
            logs.mkdir()
            for name in ("reward.txt", "reward.json"):
                (logs / name).write_text("1\n")
            process = subprocess.run([
                sys.executable, str(Path(harbor_verifier.__file__).resolve()),
                "--task", str(task), "--submission", str(submission), "--logs", str(logs),
                "--oracle", str(root / "missing-oracle"),
            ], capture_output=True, text=True)
            self.assertEqual(process.returncode, 2, process.stderr)
            report = json.loads(process.stdout)
            self.assertEqual(report["scientific_status"], "unassessed")
            self.assertIsNone(report["reward"])
            self.assertFalse((logs / "reward.txt").exists())
            self.assertFalse((logs / "reward.json").exists())
            self.assertEqual(json.loads((logs / "assessment.json").read_text()), report)

    def test_oracle_failure_cannot_be_reinterpreted_as_an_incorrect_script(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            task = root / "task.json"
            task.write_text(json.dumps({"id": "P3", "criteria": ["gap"]}))
            submission = root / "solution.in"
            submission.write_text("run 0\n")
            oracle = root / "oracle"
            oracle.write_text("fixture binary identity")
            args = argparse.Namespace(task=task, submission=submission, oracle=oracle,
                                      inputs=root, logs=root / "logs", timeout=1)
            process = subprocess.CompletedProcess([], 2, "", "engine failed to start")
            with patch.object(harbor_verifier.subprocess, "run", return_value=process):
                with redirect_stdout(io.StringIO()):
                    self.assertEqual(harbor_verifier.verify(args), 2)
            report = json.loads((args.logs / "assessment.json").read_text())
            self.assertEqual(report["scientific_status"], "unassessed")
            self.assertIsNone(report["reward"])

    def test_real_oracle_process_transports_pass_fail_and_unknown_without_inventing_physics(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            task = root / "task.json"
            task.write_text(json.dumps({"id": "P3", "criteria": ["gap"]}))
            submission = root / "solution.in"
            oracle = root / "oracle"
            oracle.write_text(
                "#!" + sys.executable + "\n"
                "import hashlib, json, pathlib, sys\n"
                "data = pathlib.Path(sys.argv[sys.argv.index('--input') + 1]).read_bytes()\n"
                "print(json.dumps({'schema_version': 1, 'task': 'P3', "
                "'submission_sha256': hashlib.sha256(data).hexdigest(), "
                "'oracle_version': 'fixture-only', 'criteria': {'gap': "
                "{'status': data.decode().strip(), 'evidence': ['protocol fixture only']}}}))\n"
            )
            oracle.chmod(0o755)
            args = argparse.Namespace(task=task, submission=submission, oracle=oracle,
                                      inputs=root, logs=root / "logs", timeout=5)
            for status, reward, expected_exit in (("pass", 1, 0), ("fail", 0, 0), ("unknown", None, 2)):
                with self.subTest(status=status):
                    submission.write_text(status + "\n")
                    with redirect_stdout(io.StringIO()):
                        self.assertEqual(harbor_verifier.verify(args), expected_exit)
                    report = json.loads((args.logs / "assessment.json").read_text())
                    self.assertEqual(report["reward"], reward)
                    reward_path = args.logs / "reward.txt"
                    self.assertEqual(reward_path.read_text() if reward_path.exists() else None,
                                     str(reward) + "\n" if reward is not None else None)

    def test_images_must_be_digest_pinned(self):
        with self.assertRaises(argparse.ArgumentTypeError):
            harbor_tasks.image("example/engine:latest")
        pinned = "example/engine@sha256:" + "a" * 64
        self.assertEqual(harbor_tasks.image(pinned), pinned)

    def test_draft_export_keeps_inputs_and_verifier_separate_and_does_not_overwrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "upstream"
            cache.mkdir()
            cases = {}
            for task_id in ("P1", "P2", "P3"):
                entry = {"name": task_id.lower(), "criteria": ["geometry"]}
                for kind, data in (("prompt", b"A fixture method.\n"), ("potential", b"fixture only\n")):
                    relative = f"{task_id}-{kind}"
                    (cache / relative).write_bytes(data)
                    entry[kind] = relative
                    entry[kind + "_blob"] = hashlib.sha1(
                        b"blob " + str(len(data)).encode() + b"\0" + data
                    ).hexdigest()
                cases[task_id] = entry
            spec = root / "tasks.json"
            spec.write_text(json.dumps({"tasks": cases, "harbor": {"version": "fixture"},
                                        "development_tasks": list(cases), "held_out_tasks": []}))
            args = argparse.Namespace(cache=cache, output=root / "dataset", condition="C",
                                      agent_image=None, verifier_image=None,
                                      agent_seconds=600, verifier_seconds=600)
            revision = harbor_tasks.evaluation.source()["commit"]
            with patch.object(harbor_tasks, "SPECIFICATION", spec):
                with patch.object(harbor_tasks, "git", return_value=revision.encode()):
                    with redirect_stdout(io.StringIO()):
                        harbor_tasks.prepare(args)
                    with self.assertRaisesRegex(ValueError, "Output already exists"):
                        harbor_tasks.prepare(args)
            report = json.loads((args.output / "preparation.json").read_text())
            self.assertFalse(report["ready_for_trials"])
            self.assertEqual({case["id"] for case in report["tasks"]}, {"P1", "P2", "P3"})
            for task_id in cases:
                directory = args.output / task_id.lower()
                environment = directory / "environment"
                self.assertFalse((environment / "verifier.py").exists())
                self.assertTrue((directory / "tests/verifier.py").is_file())
                config = (directory / "task.toml").read_text()
                self.assertIn('environment_mode = "separate"', config)
                self.assertIn('artifacts = ["/app/submission"]', config)
                self.assertIn("lamlet check INPUT --language lammps", (directory / "instruction.md").read_text())


if __name__ == "__main__":
    unittest.main()

"""Offline checks for source integrity and honest evaluation denominators."""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import evaluation


class EvaluationTests(unittest.TestCase):
    def test_selection_keeps_published_and_agent_cases_separate(self):
        self.assertEqual(evaluation.identify(
            "publication/pipeline/generated_scripts/prompt3/gpt-5.5/prompt3-gpt-5.5-T9.in"
        ), {"id": "published/gpt-5.5/P3/9", "group": "published", "task": "P3",
            "model": "gpt-5.5", "trial": 9})
        self.assertEqual(evaluation.identify(
            "publication/agent_tests/sonnet46/nb_spall_eval/nb_spall.in"
        )["id"], "agent/sonnet46/P3")
        for path in (
            "publication/pipeline/generated_scripts/prompt1/gpt-5/prompt2-gpt-5-T0.in",
            "publication/pipeline/generated_scripts/prompt1/gpt-5/prompt1-gpt-5-T10.in",
            "publication/agent_tests/sonnet46/nb_spall_eval/nb_spall_prompt.txt",
            "publication/agent_tests/sonnet46/nb_spall_eval/nb_spall_short.in",
        ):
            with self.subTest(path=path):
                self.assertIsNone(evaluation.identify(path))

    def test_missing_and_modified_files_fail_before_subsetting(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            entries = []
            for task in (1, 2, 3):
                relative = (f"publication/pipeline/generated_scripts/prompt{task}/model/"
                            f"prompt{task}-model-T0.in")
                path = cache / relative
                path.parent.mkdir(parents=True)
                data = b"run 0\n"
                path.write_bytes(data)
                blob = hashlib.sha1(b"blob 6\0" + data).hexdigest()
                entries.append(f"100644 blob {blob}\t{relative}".encode())
            specification = {"commit": "pinned", "groups": {
                "published": {"expected_files": 3, "expected_per_task": 1}}}

            def git(directory, *args):
                return b"pinned\n" if args == ("rev-parse", "HEAD") else b"\0".join(entries)

            with patch.object(evaluation, "git", side_effect=git):
                cases = evaluation.verified_inputs(cache, specification)
                case = next(case for case in cases if case["task"] == "P3")
                path = cache / case["path"]
                path.write_bytes(b"run 1\n")
                with self.assertRaisesRegex(ValueError, "Modified source input"):
                    evaluation.verified_inputs(cache, specification)
                path.unlink()
                with self.assertRaises(FileNotFoundError):
                    evaluation.verified_inputs(cache, specification)

    def test_incomplete_git_tree_fails_instead_of_changing_denominator(self):
        specification = {"commit": "pinned", "groups": {
            "published": {"expected_files": 240, "expected_per_task": 80}}}
        with patch.object(evaluation, "git", side_effect=[b"pinned\n", b""]):
            with self.assertRaisesRegex(ValueError, "Incomplete selection"):
                evaluation.verified_inputs(Path("unused"), specification)

    def test_acceptance_and_rejection_never_become_scientific_labels(self):
        accepted = {"summary": {"errors": 0, "warnings": 0, "unknowns": 4}}
        rejected = {"summary": {"errors": 1, "warnings": 0, "unknowns": 0}}
        cases = [
            {"group": "published", "task": "P3", "standard": accepted,
             "strict": accepted, "round_trip": "identical"},
            {"group": "published", "task": "P3", "standard": rejected,
             "strict": rejected, "round_trip": "build_rejected"},
            {"group": "agent", "task": "P3", "standard": accepted,
             "strict": accepted, "round_trip": "identical"},
        ]
        self.assertEqual(evaluation.summarize(cases), {
            "published/P3": {"files": 2, "standard_accepted": 1, "strict_accepted": 1,
                             "files_with_unknowns": 1,
                             "round_trip": {"build_rejected": 1, "identical": 1},
                             "scientific_assessed": 0, "scientific_correct": None},
            "agent/P3": {"files": 1, "standard_accepted": 1, "strict_accepted": 1,
                         "files_with_unknowns": 1, "round_trip": {"identical": 1},
                         "scientific_assessed": 0, "scientific_correct": None},
        })

    def test_replay_observes_actual_output_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            (cache / "script.in").write_bytes(b"run 0\n")
            case = {"id": "published/model/P1/0", "path": "script.in"}
            output = cache / "output.in"
            report = {"summary": {"errors": 0, "warnings": 0, "unknowns": 0}}

            def findings(binary, path, output=None, strict=False):
                if output is not None:
                    output.write_bytes(b"run 1\n")
                return report

            with patch.object(evaluation, "findings", side_effect=findings):
                result = evaluation.replay_case(Path("lamlet"), cache, case, output)
            self.assertEqual(result["round_trip"], "different")
            self.assertIsNone(result["scientific_correctness"])

    def test_crashed_checker_is_an_infrastructure_failure(self):
        process = subprocess.CompletedProcess([], -9, b"", b"killed")
        with patch.object(evaluation.subprocess, "run", return_value=process):
            with self.assertRaisesRegex(ValueError, "CLI failed"):
                evaluation.replay_case(Path("lamlet"), Path("unused"),
                                       {"id": "case", "path": "script.in"}, Path("output.in"))


if __name__ == "__main__":
    unittest.main()

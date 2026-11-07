"""Offline behavior checks for the corpus evaluator."""

from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import corpus


class CorpusTests(unittest.TestCase):
    def test_selection_distinguishes_inputs_from_logs_and_data(self):
        group = {"prefix": "examples/", "basename_prefix": "in."}
        paths = ["examples/a/in.one", "examples/in.two", "examples/a/log.one",
                 "bench/in.one", "examples/a/data.one"]
        self.assertEqual([p for p in paths if corpus.selected(p, group)],
                         ["examples/a/in.one", "examples/in.two"])
        self.assertTrue(corpus.selected("tutorial1/input.lammps", {"suffixes": [".lammps"]}))
        self.assertFalse(corpus.selected("tutorial1/state.data", {"suffixes": [".lammps"]}))

    def test_frequency_and_feature_markers_have_literal_expected_counts(self):
        commands, features = corpus.analyze_text(
            b'# include absent\nunits metal\ninclude settings.mod\n'
            b'jump SELF loop\nnext i\nif "$i > 1" then "run 0"\n'
            b'fix f all nve &\n option yes\nprint """one\ntwo"""\n'
        )
        # Continuation and quoted lines intentionally remain lexical candidates.
        self.assertEqual(dict(commands), {"units": 1, "include": 1, "jump": 1,
                         "next": 1, "if": 1, "fix": 1, "option": 1, "print": 1, "two": 1})
        self.assertEqual(set(features), {"include", "jump", "next", "if",
                         "variable_substitution", "continuation", "triple_quotes"})

    def test_missing_or_modified_pinned_inputs_cannot_shrink_the_denominator(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            directory = cache / "fixture"
            directory.mkdir()
            for args in (["init", "--quiet"], ["config", "user.name", "Corpus Test"],
                         ["config", "user.email", "corpus@example.invalid"]):
                corpus.git(directory, *args)
            (directory / "in.valid").write_bytes(b"run 0\n")
            (directory / "in.foreign").write_bytes(b"foreign input\n")
            corpus.git(directory, "add", "in.valid", "in.foreign")
            corpus.git(directory, "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null",
                       "commit", "--quiet", "-m", "test: create local corpus fixture",
                       "-m", "Co-Authored-By: Codex <noreply@openai.com>")
            commit = corpus.git(directory, "rev-parse", "HEAD").decode().strip()
            source = {"id": "fixture", "commit": commit,
                      "exclude": {"in.foreign": "another language"},
                      "groups": [{"id": "inputs", "basename_prefix": "in."}]}
            with patch.object(corpus, "manifest", return_value=[source]):
                items, selection = corpus.inputs(cache)
                self.assertEqual({item[2] for item in items}, {"in.valid"})
                self.assertEqual(selection, {"inputs": {"candidates": 2,
                                 "excluded": {"in.foreign": "another language"}}})
                (directory / "in.valid").write_bytes(b"run 1\n")
                with self.assertRaisesRegex(ValueError, "Modified corpus input"):
                    corpus.inputs(cache)
                (directory / "in.valid").unlink()
                with self.assertRaises(FileNotFoundError):
                    corpus.inputs(cache)

    def test_cli_io_failure_is_not_counted_as_a_validation_rejection(self):
        process = subprocess.CompletedProcess([], 2, b'{"status":"error"}', b"")
        with patch.object(corpus.subprocess, "run", return_value=process):
            with self.assertRaisesRegex(ValueError, "CLI failed"):
                corpus.findings(Path("lamlet"), Path("missing.in"))


if __name__ == "__main__":
    unittest.main()

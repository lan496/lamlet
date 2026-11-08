"""Test token meaning and exact text preservation through the public plugin API."""

import unittest
from pathlib import Path

from pygments import highlight
from pygments.formatters import HtmlFormatter
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Error, Keyword, Name, String, Text

ROOT = Path(__file__).resolve().parents[2]


class LexerTests(unittest.TestCase):
    def tokens(self, source, language="lamlet"):
        return list(get_lexer_by_name(language).get_tokens_unprocessed(source))

    def test_plugin_highlights_directives_quantities_paths_and_commands(self):
        source = '@let dt = 2 fs\n@let data: InputFile = path("Si.data")\nunits metal\ntimestep @{dt}\n'
        tokens = [(kind, value) for _, kind, value in self.tokens(source)]
        for expected in (
            (Keyword.Declaration, "@let"),
            (Name.Constant, "fs"),
            (Keyword.Type, "InputFile"),
            (Name.Function, "path"),
            (Name.Builtin, "timestep"),
            (String.Interpol, "@{"),
            (Name.Variable, "dt"),
        ):
            self.assertIn(expected, tokens)
        rendered = highlight(source, get_lexer_by_name("lamlet"), HtmlFormatter())
        self.assertIn('<span class="kd">@let</span>', rendered)
        self.assertIn('<span class="si">@{</span>', rendered)

    def test_units_are_only_semantic_inside_expressions(self):
        tokens = [
            (kind, value)
            for _, kind, value in self.tokens(
                "fix fs all nve\n@let fs_path = 1 kcal/mol\n@let fs = 2 fs\n    timestep @{fs}\n"
            )
        ]
        self.assertIn((Text, "fs"), tokens)
        self.assertIn((Name.Variable, "fs_path"), tokens)
        self.assertIn((Name.Constant, "kcal/mol"), tokens)
        self.assertIn((Name.Variable, "fs"), tokens)
        self.assertIn((Name.Builtin, "timestep"), tokens)

    def test_quoted_and_commented_delimiters_stay_literal(self):
        source = '# @{2 fs}\nprint "@{dt}"\nprint """@let t = 300 K\n@{t}"""\n'
        tokens = [(kind, value) for _, kind, value in self.tokens(source)]
        self.assertTrue(
            any(kind in Comment and "@{2 fs}" in value for kind, value in tokens)
        )
        self.assertFalse(
            any(
                kind in (Keyword.Declaration, String.Interpol, Name.Constant)
                for kind, _ in tokens
            )
        )

    def test_runtime_variables_and_opaque_commands_remain_legacy_text(self):
        tokens = [
            (kind, value)
            for _, kind, value in self.tokens(
                'custom_plugin ${runtime} "@{literal}"\n', "lammps"
            )
        ]
        self.assertIn((Name.Builtin, "custom_plugin"), tokens)
        self.assertIn((Name.Variable, "${runtime}"), tokens)
        self.assertFalse(any(kind is String.Interpol for kind, _ in tokens))

    def test_all_shipped_sources_preserve_every_character(self):
        sources = list((ROOT / "examples").rglob("*.lamlet")) + list(
            (ROOT / "demo").glob("*.lamlet")
        )
        self.assertTrue(sources)
        for path in sources:
            if not path.is_file():
                continue
            with self.subTest(path=path):
                source = path.read_text()
                tokens = self.tokens(source)
                self.assertEqual("".join(value for _, _, value in tokens), source)
                self.assertFalse(any(kind in Error for _, kind, _ in tokens))


if __name__ == "__main__":
    unittest.main()

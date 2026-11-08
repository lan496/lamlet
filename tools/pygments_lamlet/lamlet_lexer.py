"""Presentation lexers. Highlighting never determines whether input is valid."""

from typing import ClassVar

from pygments.lexer import RegexLexer, bygroups, include, words
from pygments.token import (
    Comment,
    Keyword,
    Name,
    Number,
    Operator,
    Punctuation,
    String,
    Text,
    Whitespace,
)

NUMBER = r"(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?"
UNIT = r"(?:kcal/mol|angstrom|fs|ps|ns|nm|Pa|bar|atm|eV|s|m|K)\b"


class LammpsLexer(RegexLexer):
    """Preserve opaque commands and distinguish LAMMPS runtime substitutions."""

    name = "LAMMPS"
    aliases: ClassVar[list[str]] = ["lammps"]
    filenames: ClassVar[list[str]] = ["*.lammps", "in.*"]
    tokens: ClassVar[dict] = {
        "root": [
            (r"^([ \t]*)([A-Za-z_][\w/]*)", bygroups(Whitespace, Name.Builtin)),
            (r"\n", Whitespace),
            include("literals"),
            (r"&(?=[ \t]*(?:#.*)?$)", Operator),
            (r"[^\s\"'#$&]+", Text),
            (r".", Text),
        ],
        "literals": [
            (r"[ \t]+", Whitespace),
            (r"#.*$", Comment.Single),
            (r'"""', String, "triple"),
            (r'"', String, "double"),
            (r"'", String, "single"),
            (r"\$\{[^}\n]*\}|\$[A-Za-z_]|\$\(", Name.Variable),
            (
                r"(?<![\w.])[+-]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?(?![\w.])",
                Number,
            ),
        ],
        "triple": [(r'"""', String, "#pop"), (r"[^\"]+|\"", String)],
        "double": [
            (r"\\.", String.Escape),
            (r'"', String, "#pop"),
            (r'[^"\\]+', String),
        ],
        "single": [
            (r"\\.", String.Escape),
            (r"'", String, "#pop"),
            (r"[^'\\]+", String),
        ],
    }


class LamletLexer(LammpsLexer):
    """Lamlet directives and expressions layered over permissive LAMMPS text."""

    name = "Lamlet"
    aliases: ClassVar[list[str]] = ["lamlet"]
    filenames: ClassVar[list[str]] = ["*.lamlet"]
    tokens: ClassVar[dict] = {
        **LammpsLexer.tokens,
        "root": [
            (
                r"^([ \t]*)(@(?:fix|compute|dump|drop)\b)",
                bygroups(Whitespace, Keyword.Declaration),
            ),
            (
                r"^([ \t]*)(@(?:let|for|scope|module|fn|call|stage)\b)",
                bygroups(Whitespace, Keyword.Declaration),
                "directive",
            ),
            (r"^([ \t]*)([A-Za-z_][\w/]*)", bygroups(Whitespace, Name.Builtin)),
            (r"@(?:unchecked)?\{", String.Interpol, "expression"),
            (r"\n", Whitespace),
            include("literals"),
            (r"[{}]", Punctuation),
            (r"&(?=[ \t]*(?:#.*)?$)", Operator),
            (r"[^\s\"'#$@{}&]+", Text),
            (r".", Text),
        ],
        "directive": [
            (r"\n", Whitespace, "#pop"),
            (r"[ \t]+", Whitespace),
            (r"#.*$", Comment.Single),
            (r"@(?:unchecked)?\{", String.Interpol, "expression"),
            include("expr-tokens"),
            (r".", Text),
        ],
        "expression": [
            (r"\}", String.Interpol, "#pop"),
            include("expr-tokens"),
            (r".", Text),
        ],
        "expr-tokens": [
            (r"[ \t]+", Whitespace),
            (r'"', String, "double"),
            (words(("in", "requires", "ensures"), suffix=r"\b"), Keyword),
            (words(("true", "false"), suffix=r"\b"), Keyword.Constant),
            (
                words(
                    (
                        "Int",
                        "Float",
                        "Bool",
                        "Time",
                        "Length",
                        "Temperature",
                        "Pressure",
                        "Energy",
                        "InputFile",
                        "OutputFile",
                        "Directory",
                        "InputScript",
                        "PotentialFile",
                        "InputPattern",
                        "DynamicPath",
                    ),
                    suffix=r"\b",
                ),
                Keyword.Type,
            ),
            (
                words(
                    ("path", "output", "dir", "pattern", "dynamic"), suffix=r"(?=\s*\()"
                ),
                Name.Function,
            ),
            (
                f"({NUMBER})([ \\t]+)({UNIT})",
                bygroups(Number, Whitespace, Name.Constant),
            ),
            (NUMBER, Number),
            (r"[A-Za-z_]\w*", Name.Variable),
            (r"[+*/=\-]", Operator),
            (r"[\[\](){},.:]", Punctuation),
        ],
    }

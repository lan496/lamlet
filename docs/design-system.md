# Design tokens and syntax highlighting

The documentation uses a scientific notebook style: warm paper, dark ink,
oxide red, serif headings, monospace annotations, and thin rules. Sections use
numbered labels and aligned rows. The layout has square corners and no shadows.

## Token source

[`tokens.css`](https://github.com/lan496/lamlet/blob/main/docs/assets/stylesheets/tokens.css)
contains the shared values. `extra.css` consumes those values for Zensical and
the landing page. Both stylesheets load through `zensical.toml`.

<div class="lamlet-token-swatches">
  <span class="token-paper">Paper</span>
  <span class="token-ink">Ink</span>
  <span class="token-oxide">Oxide</span>
</div>

| Token group | Role |
| --- | --- |
| `--lamlet-color-*` | Page, surface, text, rules, accent, focus, and selection |
| `--lamlet-font-*` | Body, display, and code font stacks |
| `--lamlet-type-*` | Small labels, body text, lead, and display sizes |
| `--lamlet-space-*` | Shared spacing scale |
| `--lamlet-rule*`, `--lamlet-radius` | Borders and corner shape |
| `--lamlet-syntax-*` | Directive, command, type, unit, number, string, variable, and comment colors |
| `--lamlet-duration` | Interaction timing, disabled for reduced motion |

Light and dark modes use the same semantic names. Components contain no literal
colors. Font stacks use locally installed fonts and do not require a font CDN.
The single 760 px layout breakpoint stacks the hero and code comparison and
reduces feature rows to two columns.

## Highlight a Lamlet example

Use a `lamlet` code fence in a documentation page:

```lamlet
# Quantities and input resources
@let dt = 2 fs
@let structure: InputFile = path("structures/Si.data")
units metal
read_data @{structure}
timestep @{dt}

@scope {
    @fix integrate all nve
    run 100
}
```

Directives and interpolation delimiters use oxide red. Types use violet, units
use ochre, commands and constructors use blue, and strings use green. Colors
supplement the original spelling and punctuation.

Use a `lammps` fence for generated or legacy input:

```lammps
# Generated LAMMPS
units metal
timestep 0.002
run ${steps}
print "@{this stays literal}"
```

The local `lamlet-pygments` package registers both lexers through
[Pygments entry points](https://pygments.org/docs/plugins/). Zensical renders the
colored spans during the build. There is no browser highlighter or compiler in
the page. Installing `docs/requirements.txt` installs the local plugin.

Highlighting is presentation only. It preserves unknown command text and does
not validate types, effects, or physical meaning. The site's highlighter does
not add Lamlet highlighting to GitHub's Markdown renderer or to an editor.

## Verify a change

From the repository root:

```sh
uv run --with-requirements docs/requirements.txt python -m unittest discover -s tools/pygments_lamlet -p 'test_*.py'
uv run --with-requirements docs/requirements.txt zensical build --strict
python3 tools/check_docs.py target/docs-site
```

The lexer tests check directives, quantities, paths, literal delimiters in
comments and strings, runtime variables, opaque commands, and exact text
preservation across the shipped Lamlet examples. Review both color schemes at
desktop and mobile widths after changing layout or color tokens.

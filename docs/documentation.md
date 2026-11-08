# Build and preview the documentation

The English documentation uses [Zensical](https://zensical.org/), pinned in
`docs/requirements.txt`. Run these commands from the repository root with
[uv](https://docs.astral.sh/uv/) installed.

## Preview locally

```sh
uv run --with-requirements docs/requirements.txt zensical serve
```

Open `http://127.0.0.1:8000`. Zensical rebuilds pages when you edit the Markdown
or the configuration. The landing page takes its input/output presentation and
gradual adoption sequence from the [TypeScript site](https://www.typescriptlang.org/).

## Build the static site

```sh
uv run --with-requirements docs/requirements.txt zensical build --strict
python3 tools/check_docs.py target/docs-site
```

The output is `target/docs-site/`. The link checker verifies local destinations
and anchors in the built HTML. It does not check external sites.
CI builds the site and uploads it as the `lamlet-documentation` artifact.

`zensical.toml` defines the navigation, theme, and intended GitHub Pages URL.
Building the site does not publish it. To serve a built copy locally:

```sh
python3 -m http.server 8000 --directory target/docs-site
```

## Publish on GitHub Pages

The public documentation is at [lan496.github.io/lamlet](https://lan496.github.io/lamlet/).
The [CI workflow](https://github.com/lan496/lamlet/actions/workflows/ci.yml)
deploys the checked site after updates to `main`. Rust checks on Linux, macOS,
and Windows, corpus checks, and documentation checks must all pass first.
Pull requests build and validate the site without publishing it.

Maintainers can also select **Run workflow** on the CI Actions page, or run:

```sh
gh workflow run ci.yml --ref main
```

The `github-pages` environment limits deployment to `main`. The Pages settings
use **GitHub Actions** as the publishing source. Deployment uses the built
`target/docs-site/` artifact with the job-scoped `pages: write` and
`id-token: write` permissions; no personal access token is required.

## Update the pages

Edit the existing guides in `docs/`. Keep commands consistent with `lamlet --help`
and run the linked examples after changing them. Pages link to repository files
on GitHub when those files are outside the documentation tree.

The [demo guide](demo.md) explains how to regenerate the video and diagram assets.
The [design system reference](design-system.md) describes the shared tokens and
the build-time Lamlet and LAMMPS syntax highlighters.

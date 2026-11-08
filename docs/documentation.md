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

## Update the pages

Edit the existing guides in `docs/`. Keep commands consistent with `lamlet --help`
and run the linked examples after changing them. Pages link to repository files
on GitHub when those files are outside the documentation tree.

The [demo guide](demo.md) explains how to regenerate the video and diagram assets.

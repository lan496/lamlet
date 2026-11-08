# Review the lint reference labels

The [review packet](assets/lint-review/packet.md) contains all 45 fixed corpus
inputs, their single-edit mutation differences, and pinned manual references.
It includes the author's proposed labels in collapsed sections. It contains no
observed checker output. The [review form](assets/lint-review/review.json) is an
uncompleted template. Independent expert review is pending.

Read the applicable condition and input before opening the proposed label.
Judge that condition under the fragment's stated prerequisites. These fragments
are not complete simulations. An intentional advisory condition can be labelled
positive without constituting a scientific error. An absent finding differs
from an unresolved finding.

Save a copy of the form. Enter your name, affiliation, date, and whether you are
independent of both the implementation and the proposed labels. For every case,
replace `pending` with `agree`, `disagree`, or `unsure` and give a reason grounded
in the cited manual or an explicit physical argument. Keep the IDs and hashes.
Submit the form and any proposed counterexamples in
[Issue #2](https://github.com/lan496/lamlet/issues/2).

Validate a completed form with:

```sh
python3 tools/lint_review.py validate --review target/review.json
```

Exit code 0 means all labels have agreement with an independence attestation.
Exit code 1 means the review is pending or contains disagreement or uncertainty.
Exit code 2 means the form or corpus is invalid. The tool checks completeness and
hashes; it cannot verify a reviewer's identity, expertise, or independence.
Disagreement requires adjudication and a versioned label change, rather than
editing expectations to match checker output. A completed review does not
establish a population false-positive rate or scientific validity of a simulation.

Regenerate the packet without invoking Lamlet:

```sh
python3 tools/lint_review.py prepare --output target/lint-review
```

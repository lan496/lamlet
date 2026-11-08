# Independent lint-label review

Review the stated condition, not whether a fragment is a complete simulation.
Distinguish documented advisories from errors. Record a judgement before opening
the author's proposed label. No observed Lamlet output is included.

Corpus SHA-256: `9cc6acce877677ded13d0ea56a4bd2072e790071326d38949a25e62590e37060`.

Return a completed copy of `review.json`. Use `agree`, `disagree`, or `unsure`,
with a reason for every decision. Pending entries do not count as review.

## averaging-boundary

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time 2 5 10 c_thermal
```

Input SHA-256: `e689eda0a38bb059cc68be67c68251f91ec8a0a4cc81cdab77d2bf972be5f708`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

The window equals the output frequency. Equality is allowed, so a strict less-than rule would be overbroad.

</details>

## averaging-divisibility

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time 3 2 10 c_thermal
```

Input SHA-256: `799c36d39953d4acf74674d4cbbf5b5cba8c4ee033b1a10362989300fbfd4eb5`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

Single edit relative to `averaging-spaced`:

```diff
--- averaging-spaced
+++ averaging-divisibility
@@ -1,2 +1,2 @@
 compute thermal all temp
-fix sample all ave/time 2 2 10 c_thermal
+fix sample all ave/time 3 2 10 c_thermal
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The window still fits, but 10 is not divisible by the new stride 3.

</details>

## averaging-other-style

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/correlate 2 6 10 c_thermal
```

Input SHA-256: `1a358832346a21c3df30b2f1065f1a6b6ea64511e20e76334d3f07c336983c43`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `out_of_scope`. Expected target outcomes: `[]`.

For default ave one, ave/correlate requires (Nrepeat-1)*Nevery <= Nfreq. Here 10 <= 10 is valid, although the same counts would violate ave/time. The ave/time finding must be absent.

</details>

## averaging-runtime-repeat

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time 2 ${repeat} 10 c_thermal
```

Input SHA-256: `ac3e973044c87fe0378eae94b43beee89456c0665a2fe2644d07d258e5490f45`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The missing repeat count prevents deciding positivity or the window bound.

</details>

## averaging-runtime-stride

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time ${stride} 2 10 c_thermal
```

Input SHA-256: `eed02f7479d72f5ffb02b03d10409d493dbc26e4f6b0b323fb4bcbe7614aeb09`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The missing runtime stride prevents deciding divisibility or the window bound.

</details>

## averaging-spaced

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time 2 2 10 c_thermal
```

Input SHA-256: `c4b7f4fd7e3f978d52ef3afad7c50cdeabc2960c26819b6c315f32ef6041fde4`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

All counts are positive, 10 is divisible by 2, and the four-step sampling window fits within ten steps.

</details>

## averaging-window

Rule: `LML-MAN-AVE-TIME` (constraint).

Positive sampling intervals, Nfreq divisible by Nevery, and Nevery*Nrepeat <= Nfreq for ave/time.

```lammps
compute thermal all temp
fix sample all ave/time 2 6 10 c_thermal
```

Input SHA-256: `b2756ba191b154f5dbb07b0a7e1fa026b264264168082f9cafd071c6d2b49082`.

References:

- [fix_ave_time.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_time.rst) (SHA-256 `034028817518dfeb2f6f9139758a38f58000e66faea318a234681eeb811455f3`)
- [fix_ave_correlate.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_ave_correlate.rst) (SHA-256 `f5916069752fe52b4d874fef029555b26997c5292d94442175bcb7c98f790b34`)

Single edit relative to `averaging-boundary`:

```diff
--- averaging-boundary
+++ averaging-window
@@ -1,2 +1,2 @@
 compute thermal all temp
-fix sample all ave/time 2 5 10 c_thermal
+fix sample all ave/time 2 6 10 c_thermal
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The frequency remains divisible by the stride, but the twelve-step window exceeds ten steps.

</details>

## drag-default

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1
```

Input SHA-256: `a7d992d74f54716675db29101a8b8e6c202c7908eab52021b734ad13f1da7ee1`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

No drag keyword uses the documented zero default.

</details>

## drag-extension

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1 extension drag 1
```

Input SHA-256: `39f087d2f4de51875617ef41985aac352142d274973d234948ca69b194e66543`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The unreviewed option prevents identifying argument roles reliably.

</details>

## drag-fractional

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1 drag 0.1
```

Input SHA-256: `c3a45c37e38aef91126354f69d6cd120d96ae32b645534b351306ead0ecf0df8`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

Single edit relative to `drag-zero`:

```diff
--- drag-zero
+++ drag-fractional
@@ -1 +1 @@
-fix ensemble all nvt temp 300 300 0.1 drag 0
+fix ensemble all nvt temp 300 300 0.1 drag 0.1
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

A small positive drag still modifies nominal sampling; magnitude is not an exemption.

</details>

## drag-group-name

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all npt temp 300 300 0.1 iso 0 0 1 dilate drag
```

Input SHA-256: `f43e388093831885cb41840507f1eed16b3ebceba4d90a8b2d3b63fe9e1528d2`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

Here drag names the externally supplied dilate group, not a drag keyword.

</details>

## drag-nonzero

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1 drag 1
```

Input SHA-256: `ce6d2b371098a909c5bdef43c83983cd75798608f825318ceccac37c385c4174`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

Single edit relative to `drag-zero`:

```diff
--- drag-zero
+++ drag-nonzero
@@ -1 +1 @@
-fix ensemble all nvt temp 300 300 0.1 drag 0
+fix ensemble all nvt temp 300 300 0.1 drag 1
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

Nonzero drag changes the nominal ensemble distribution. The advisory does not label the physical intent incorrect.

</details>

## drag-runtime

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1 drag ${drag}
```

Input SHA-256: `8eac19d482c6a20ef08190beaf91ec4a12886e297fbacbc7503b26ae47d00124`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The drag factor is not statically supplied.

</details>

## drag-zero

Rule: `LML-MAN-NH-DRAG` (advisory).

Nonzero drag changes nominal Nose-Hoover sampling; intentional drag remains valid.

```lammps
fix ensemble all nvt temp 300 300 0.1 drag 0
```

Input SHA-256: `a21f7654ff2b5a7be76e769a453383a8c6f92170fa600b5c1114242031099d37`.

References:

- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

A zero drag factor contributes no additional drag term.

</details>

## integers-axis

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice fcc 4.05 orient x 1 0 0
```

Input SHA-256: `fb9e6856cdb3f6db60c7b5f84872e53dab7fa4ac773760fe0d0ae4d24756594d`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

The orient direction is the integer triple (1,0,0).

</details>

## integers-decimal

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice fcc 4.05 orient x 1.5 0 0
```

Input SHA-256: `40a4692ea1eccaee8da20b7d5aa32e41f1708596ea117a7bc0a221b7ab13183b`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

Single edit relative to `integers-axis`:

```diff
--- integers-axis
+++ integers-decimal
@@ -1 +1 @@
-lattice fcc 4.05 orient x 1 0 0
+lattice fcc 4.05 orient x 1.5 0 0
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

A nonintegral decimal component cannot specify an integer lattice direction.

</details>

## integers-extension

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice extension/style 4.05 orient x 0.8 0 0
```

Input SHA-256: `f8316f50867568476d35c72dad9d0d3f0e3bf720ee910fcd7d5b27725981634f`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

An unreviewed extension style may define different arguments; core lattice rules cannot establish validity.

</details>

## integers-fraction

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice fcc 4.05 orient x 0.8 0 0
```

Input SHA-256: `26f63fd15945e0776d9e64c271fa977c4d649fb358c3f218ff0cb66053e43a0f`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

Single edit relative to `integers-axis`:

```diff
--- integers-axis
+++ integers-fraction
@@ -1 +1 @@
-lattice fcc 4.05 orient x 1 0 0
+lattice fcc 4.05 orient x 0.8 0 0
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

A fractional orient component violates the documented integer requirement.

</details>

## integers-primitive

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice custom 3.52 a1 1.0 0.0 0.0 a2 0.5 1.0 0.0 a3 0.0 0.0 0.5 basis 0.0 0.0 0.0
```

Input SHA-256: `57ee970d9774246d8c711395727fbdd65fb42f1d6483e263b171360f1cb0a9bc`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

Primitive vectors may have real and nonorthogonal components; the integer restriction concerns orient.

</details>

## integers-runtime

Rule: `LML-MAN-LATTICE-ORIENT-INTEGERS` (constraint).

Integer lexical directions in orient, distinct from real primitive vectors.

```lammps
lattice fcc 4.05 orient x ${axis} 0 0
```

Input SHA-256: `a83821b59289275091a47983ab9b67e40c377402dbfc187d988588a34471b0b5`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The runtime expansion may be either integer or fractional; no value is supplied.

</details>

## integration-barostat

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first all nve
fix second all nph iso 0 0 1
run 10
```

Input SHA-256: `646c79293797f06a27c6be72d28433b4d598938371ba3ce839e8ad07a4607e9b`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

Single edit relative to `integration-langevin`:

```diff
--- integration-langevin
+++ integration-barostat
@@ -1,3 +1,3 @@
 fix first all nve
-fix second all langevin 300 300 0.1 12345
+fix second all nph iso 0 0 1
 run 10
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

Replacing force-only Langevin with time-integrating nph makes both active fixes integrate all atoms.

</details>

## integration-duplicate

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first all nve
fix second all nvt temp 300 300 0.1
run 10
```

Input SHA-256: `57a71dca44163ebf5545c84f153066f25eae5b1de0d6ad63574ee7bf9aaeea82`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

Single edit relative to `integration-sequential`:

```diff
--- integration-sequential
+++ integration-duplicate
@@ -1,4 +1,3 @@
 fix first all nve
-unfix first
 fix second all nvt temp 300 300 0.1
 run 10
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

Both nve and nvt update the same nonempty all group during the positive run.

</details>

## integration-langevin

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first all nve
fix second all langevin 300 300 0.1 12345
run 10
```

Input SHA-256: `4541d1533e80e1753d89f91a85e256d91d4d4230fadbbbdd9f2eaaa7f5f0c8f0`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `[]`.

Langevin modifies forces and requires an integrator; combining it with nve does not integrate twice.

</details>

## integration-membership

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first left nve
fix second right nve
run 10
```

Input SHA-256: `564936a6effde134d70b163fb6e90e3038efe42bf959483aa061e6d0cb35789c`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The externally supplied left and right groups may overlap; their membership is unspecified.

</details>

## integration-plugin

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first all nve
fix second all nve
fix callback all extension/integrator
run 10
```

Input SHA-256: `47659903bdd2354fb4f58a66eafdb780721a345a0964009d90eb0049a6849146`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The unreviewed callback may change integration state; an ordinary overlap conclusion requires known effects.

</details>

## integration-sequential

Rule: `LML-PHYS-001` (advisory).

Ordinary integrators on known nonempty overlapping groups during a positive run; specialized schemes require review.

```lammps
fix first all nve
unfix first
fix second all nvt temp 300 300 0.1
run 10
```

Input SHA-256: `c4082e24925dfb7030b80fcf3b4e6bc4a4374d562feeaed1fcb542698c6209dc`.

References:

- [fix_nve.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nve.rst) (SHA-256 `0f2b352562a1e6754418ff50b925159964932bcc307dc1983dc247fbd1b40ff3`)
- [fix_nh.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_nh.rst) (SHA-256 `4dc2494c1e863568486db21b9496a4bb3bf162b3f2a7eb0827833a0864be07e3`)
- [fix_langevin.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_langevin.rst) (SHA-256 `ba1e8a756c8471d01546107ac8210800e55108d4e4dd1829d0aebae8f8db5851`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `[]`.

The first integrator is removed before the second is defined; no simultaneous duplicate exists.

</details>

## kspace-active

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style pppm 1e-4
neigh_modify exclude type 1 2
run 0
```

Input SHA-256: `b786a51dd6fcfa0542970e8cfdfebabc03f2dc0fe8383abdbc5657e60a48f8f5`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

Single edit relative to `kspace-cleared`:

```diff
--- kspace-cleared
+++ kspace-active
@@ -1,4 +1,3 @@
 kspace_style pppm 1e-4
 neigh_modify exclude type 1 2
-neigh_modify exclude none
 run 0
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

Removing the reset leaves excluded type pairs with active PPPM; the manual warns that long-range pair corrections are not provided by exclude.

</details>

## kspace-cleared

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style pppm 1e-4
neigh_modify exclude type 1 2
neigh_modify exclude none
run 0
```

Input SHA-256: `da38b9d59cf39f97da0f040b45204dc1e81377b3f7ab2f38472b3fb7ba9ca552`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

exclude none removes all exclusions before the execution boundary.

</details>

## kspace-disabled

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style none
neigh_modify exclude type 1 2
run 0
```

Input SHA-256: `691ea6d7976630e8a71820730ab967b93e3804fa3161ca30eaa36a3a06906a53`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

No active long-range solver remains, so this particular warning condition is absent.

</details>

## kspace-ewald

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style ewald 1e-4
neigh_modify exclude type 1 2
run 0
```

Input SHA-256: `a1b5e1ea8a666e6ca388ab8a85e1a4f00fef9aaffb242b4f2c82d2135e29d127`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

Single edit relative to `kspace-disabled`:

```diff
--- kspace-disabled
+++ kspace-ewald
@@ -1,3 +1,3 @@
-kspace_style none
+kspace_style ewald 1e-4
 neigh_modify exclude type 1 2
 run 0
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

Activating Ewald while retaining excluded type pairs introduces the documented correction concern.

</details>

## kspace-extension

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style extension/solver 1e-4
neigh_modify exclude type 1 2
run 0
```

Input SHA-256: `46014fbd6c2c2fac9bd8a852a059fac38f8aa7e5186eaef1666bdff0fbd627f0`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

An unreviewed solver need not use the Ewald or PPPM correction scheme.

</details>

## kspace-runtime

Rule: `LML-MAN-KSPACE-EXCLUSIONS` (advisory).

Active neighbor exclusions with reviewed Ewald or PPPM at execution require long-range correction review.

```lammps
kspace_style pppm 1e-4
neigh_modify exclude type ${type} 2
run 0
```

Input SHA-256: `eb99f3d1350257574b2bf1b7097ac0042d434fdadf870511429c7686f54c8da5`.

References:

- [neigh_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/neigh_modify.rst) (SHA-256 `93efff503d9c425999a3ff2da5e3054082031f43ddd6119bde32cfed8b283670`)
- [kspace_style.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/kspace_style.rst) (SHA-256 `641ebaae7483fb41f6b04ef4e6e3d0d574dce0353f6cd9182be2219228f1bd61`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

An unresolved type expansion prevents establishing the final exclusions.

</details>

## orientation-default

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05
```

Input SHA-256: `456ec078775d0f52ed346d2b108807b3576797a899ff2d525c7e704e55270a15`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

Default orientation is the right-handed identity basis.

</details>

## orientation-large

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 1 orient x 9223372036854775807 0 0 orient y 0 9223372036854775807 0 orient z 0 0 9223372036854775807
```

Input SHA-256: `bd3e68f1c58f8b00145ae6a08800b62261c63f14918c10b45b60c571be8d7f48`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The mathematical basis is orthogonal and right-handed, but its determinant exceeds checked i128 arithmetic. This is an explicit analysis limit, not a physical defect.

</details>

## orientation-last-option

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05 orient x 1 1 0 orient x 1 0 0
```

Input SHA-256: `dc2e10fd9cef8a3b2b1c5100625fa6dee71eb8986cf434145d2e00524ae3c01a`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

The last x orientation restores the identity basis; earlier integer values do not set the final geometry.

</details>

## orientation-left-handed

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05 orient z 0 0 -1
```

Input SHA-256: `18e94c297e4f740482f542bd1a1a6bf3a4cb8a31a31548c94ba7c11319e68157`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

Single edit relative to `orientation-default`:

```diff
--- orientation-default
+++ orientation-left-handed
@@ -1 +1 @@
-lattice fcc 4.05
+lattice fcc 4.05 orient z 0 0 -1
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The determinant changes from +1 to -1 while pairwise orthogonality is preserved.

</details>

## orientation-oblique

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05 orient x 4 3 0 orient y -2 4 0 orient z 0 0 7
```

Input SHA-256: `1d32f203bc339f0415c72062dc67bdb87f6f5fbb84283f7c08a1cc5de6ab9f2b`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

Single edit relative to `orientation-rotated`:

```diff
--- orientation-rotated
+++ orientation-oblique
@@ -1 +1 @@
-lattice fcc 4.05 orient x 4 3 0 orient y -3 4 0 orient z 0 0 7
+lattice fcc 4.05 orient x 4 3 0 orient y -2 4 0 orient z 0 0 7
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The modified x-y dot product is 4, so the final directions are not orthogonal.

</details>

## orientation-rotated

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05 orient x 4 3 0 orient y -3 4 0 orient z 0 0 7
```

Input SHA-256: `45976a3f67ecfa2f492fd5297d6f9b1dc310af53099ba4a13650608c81ec5649`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

The x-y dot product is zero and the determinant is 175, so the integer basis is right-handed.

</details>

## orientation-runtime

Rule: `LML-MAN-LATTICE-ORIENTATION` (constraint).

Orthogonal, right-handed integer orient triples.

```lammps
lattice fcc 4.05 orient x 1 ${axis} 0
```

Input SHA-256: `3522dbe2e0c6e477de0e3d39eaa6b6e83286eeb9fa56a37b221116c2ed1bebdb`.

References:

- [lattice.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/lattice.rst) (SHA-256 `ca564e5e56c894d8cd9439d347ab33e8aad997d927194c38e8580374ce95aadf`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

The unknown component controls orthogonality; no value is supplied.

</details>

## temperature-default-output

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/com
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp thermo_temp
run 10
```

Input SHA-256: `15c1b3921ab92435c53ca5be082ab0f5e7158b8f1df1a105174fcaaabd94fe0b`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

Single edit relative to `temperature-shared`:

```diff
--- temperature-shared
+++ temperature-default-output
@@ -1,5 +1,5 @@
 compute thermal all temp/com
 fix ensemble all nvt temp 300 300 0.1
 fix_modify ensemble temp thermal
-thermo_modify temp thermal
+thermo_modify temp thermo_temp
 run 10
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The thermostat subtracts COM velocity while the default output temperature does not. Intentional differences remain allowed.

</details>

## temperature-distinct-bias

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/com
compute observed all temp
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp observed
run 10
```

Input SHA-256: `62adb4df8b56b1bb040b5a41d066e04139b611520898d5186f879c153b59c5c7`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

Single edit relative to `temperature-equivalent`:

```diff
--- temperature-equivalent
+++ temperature-distinct-bias
@@ -1,5 +1,5 @@
 compute thermal all temp/com
-compute observed all temp/com
+compute observed all temp
 fix ensemble all nvt temp 300 300 0.1
 fix_modify ensemble temp thermal
 thermo_modify temp observed
```

<details>
<summary>Author's proposed label and rationale</summary>

Label: `positive`. Expected target outcomes: `["violated"]`.

The displayed compute now uses a different velocity bias definition on the same group.

</details>

## temperature-equivalent

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/com
compute observed all temp/com
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp observed
run 10
```

Input SHA-256: `8f0162d69317d5d26f45119d86c9cd905262781a8e1cc1dc47ee70f540e6e4f3`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

Different compute IDs with the same group and unmodified temp/com definition have equivalent modeled velocity bias.

</details>

## temperature-extension

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/partial 1 1 0
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp thermal
run 10
```

Input SHA-256: `770ddf40f8cbf2a4c74ba63462af9996b1b3e84c12e0fb3ef5c31ff08a7e0341`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

Partial-component temperature is a valid documented alternative but is outside the modeled temp and temp/com comparison.

</details>

## temperature-modified

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/com
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp thermal
compute_modify thermal extra/dof 0
run 10
```

Input SHA-256: `6917ff011cd9991f3a1588151b1f89f1c77ce50115dc5d14e4571ece69efa82c`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `unknown`. Expected target outcomes: `["unknown"]`.

A compute_modify change alters a temperature definition outside this check's modeled policy.

</details>

## temperature-shared

Rule: `LML-PHYS-002` (advisory).

Same-group thermostat and displayed temperature use known but different velocity bias definitions.

```lammps
compute thermal all temp/com
fix ensemble all nvt temp 300 300 0.1
fix_modify ensemble temp thermal
thermo_modify temp thermal
run 10
```

Input SHA-256: `3363effe60458462818bdfcde07bf7fa73b1ad53bf54c83a43882fc46a445daa`.

References:

- [compute_temp.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp.rst) (SHA-256 `98637a6b155b9d33242ca46c68d6be70bdabbcf5223b5b18f40ec71274e8b095`)
- [compute_temp_com.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/compute_temp_com.rst) (SHA-256 `5f5cfc216059396c45467b91c8eec6f317d8c800fafd48b3f58454abfb5c266c`)
- [fix_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/fix_modify.rst) (SHA-256 `40c4cde9d5886307550d2b021a6b9a5fdfa1978da4d11ce93b43984f179b57e2`)
- [thermo_modify.rst](https://github.com/lammps/lammps/blob/0165101ce505734e8fb249694b22e295a7ab4182/doc/src/thermo_modify.rst) (SHA-256 `12ead71e14028340147519a9eb402bceeb77f04c5aeb392f1e3c46fc9e0cbe94`)

<details>
<summary>Author's proposed label and rationale</summary>

Label: `negative`. Expected target outcomes: `["satisfied"]`.

Both thermostat and displayed temperature select the same COM-subtracted compute.

</details>

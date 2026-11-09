# Review of bounded natural-report conditions

Assess the condition evidence and provenance scope. No checker observations are included.
Agreement concerns the bounded assessment, not a gold label or complete simulation validity.
Return review.json with agree, disagree, or unsure and a reason for every case.

The native criteria use relative tolerance 1e-8. The GCMC comparison uses
T = 2 K / [k_B (3 N - 3)], with k_B = 0.0019872067 kcal/mol/K, no molecular
constraints, and a fluid group containing all atoms. K and N come from native stdout.
The velocity reconstruction prescribes 100 Angstrom/ps for one atom of mass 183.84.
Its expected energy is m v^2 / 2 times the metal-unit conversion 0.00010364269.
This external speed contract is not a reproduction of the original 20 eV collision.

[Frozen native plan](https://github.com/lan496/lamlet/blob/main/research/defects/replays.json)
and [velocity template](https://github.com/lan496/lamlet/blob/main/research/defects/velocity.in.template).

## orientation

Argument acceptance for one extracted command; reconstructed integer-scaled correction, no complete crystal.

Intent: Rotate an FCC lattice using the stated orthogonal directions.

Source: [m18394](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394). Provenance: `source_fragment_with_reconstructed_correction`.

Source evidence:

- [Post 1](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394/1): report. 0.8 0.6 0
- [Post 2](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394/2): diagnosis. must be integer values
- [Post 3](https://matsci.org/t/lattice-orient-these-vectors-must-be-orthogonal/18394/3): confirmation. saved me an hour

Proposed change:

```diff
-lattice fcc 4.05 orient x 0.8 0.6 0 orient y -0.6 0.8 0 orient z 0 0 1
+lattice fcc 4.05 orient x 4 3 0 orient y -3 4 0 orient z 0 0 1
```

### before

Input SHA-256: `6b2044f318fdc2814b095b49739ff3add2f4d77fe3cd3722df51bc1428feb62c`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/orientation/before/engine.stdout).

```json
{
  "engine_exit_code": 1,
  "timed_out": false,
  "measurement": null,
  "proposed_assessment": "violated"
}
```

### after

Input SHA-256: `7f9d7da9579c3730b72c1caa864dde4cef3cffa1b9ee9ae3afe3cf3d1a9d9394`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/orientation/after/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": null,
  "proposed_assessment": "satisfied"
}
```

Dependencies and limitations:

- Original full script and version are absent.
- The reported error text can differ in modern versions.
- Any complete simulation or explicit corrected command must be labelled reconstructed.

## averaging

Historical write_dump failure did not reproduce on the pinned modern engine. Successful execution is not a physical-validity label.

Intent: Write time-averaged per-atom values after a run.

Source: [m34056](https://matsci.org/t/fix-ave-atom-issues-deciphering-dump-custom-and-fix-not-at-compatible-times/34056). Provenance: `complete_attachment_with_proposed_correction`.

Source evidence:

- [Post 3](https://matsci.org/t/fix-ave-atom-issues-deciphering-dump-custom-and-fix-not-at-compatible-times/34056/3): report. write_dump all custom foo2
- [Post 4](https://matsci.org/t/fix-ave-atom-issues-deciphering-dump-custom-and-fix-not-at-compatible-times/34056/4): diagnosis. for technical reasons

Proposed removal:

```diff
-write_dump all custom foo2 id type mass f_avg[*]
```

### before

Input SHA-256: `829b3a1b7d24ff982bf1299bb145b7a5eeb44cb23e976200a8afd33169d3ebfa`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/averaging/before/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": null,
  "proposed_assessment": "execution_only"
}
```

### after

Input SHA-256: `cbcbc711816fb6175722c41e563c739d3ffa2d8455fbf2944ec50ce87254b6fc`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/averaging/after/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": null,
  "proposed_assessment": "execution_only"
}
```

Dependencies and limitations:

- Historical executable version is not specified.
- Modern behavior may differ from the reported 2020 implementation.

## gcmc-original

Both million-step variants timed out. Static observations are retained but excluded from native-condition comparisons.

Intent: Maintain the prescribed temperature during GCMC insertion and deletion.

Source: [m37447](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447). Provenance: `complete_inline_with_reported_correction`.

Source evidence:

- [Post 2](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447/2): correction. compute_modify  NVT_temp dynamic/dof yes
- [Post 3](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447/3): confirmation. works perfectly as intended

Proposed change:

```diff
-fix_modify      NVT dynamic/dof yes
+compute_modify  NVT_temp dynamic/dof yes
```

### before

Input SHA-256: `ae609861e7e5e9e241ef4a67d8d6027471c14549ae41b2311a71337511d5d718`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/gcmc-original/before/engine.stdout).

```json
{
  "engine_exit_code": null,
  "timed_out": true,
  "measurement": null,
  "proposed_assessment": "unassessed"
}
```

### after

Input SHA-256: `5fc6f13881c3e9485c25a52e2b14928d285b527006c80a39f6409a1d05484c1f`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/gcmc-original/after/engine.stdout).

```json
{
  "engine_exit_code": null,
  "timed_out": true,
  "measurement": null,
  "proposed_assessment": "unassessed"
}
```

Dependencies and limitations:

- Historical executable unavailable.
- GCMC trajectory and temperature behavior need independent verification.

## gcmc-instrumented

A 400-step derivative tests the thermostat compute normalization, not equilibration or the reported long-time freezing.

Intent: Maintain the prescribed temperature during GCMC insertion and deletion.

Source: [m37447](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447). Provenance: `shortened_instrumented_replay`.

Source evidence:

- [Post 2](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447/2): correction. compute_modify  NVT_temp dynamic/dof yes
- [Post 3](https://matsci.org/t/gcmc-of-confined-lj-fluid/37447/3): confirmation. works perfectly as intended

Proposed change:

```diff
-fix_modify      NVT dynamic/dof yes
+compute_modify  NVT_temp dynamic/dof yes
```

### before

Input SHA-256: `4657b4decaec307855b303abec222b0d80cb5869774fcbce1f69ac69dc106779`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/gcmc-instrumented/before/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": {
    "atom_count_changed": true,
    "max_relative_error": 65.0,
    "samples": [
      {
        "atoms": 2,
        "kinetic_temperature_K": 77.0,
        "relative_error": 0.0,
        "step": 0,
        "thermostat_temperature_K": 77.0
      },
      {
        "atoms": 38,
        "kinetic_temperature_K": 1.704781926688939,
        "relative_error": 35.99999999999999,
        "step": 100,
        "thermostat_temperature_K": 63.07693128749074
      },
      {
        "atoms": 38,
        "kinetic_temperature_K": 0.4196818158000477,
        "relative_error": 35.99999999999999,
        "step": 200,
        "thermostat_temperature_K": 15.528227184601763
      },
      {
        "atoms": 67,
        "kinetic_temperature_K": 37.75376530458098,
        "relative_error": 65.0,
        "step": 300,
        "thermostat_temperature_K": 2491.748510102345
      },
      {
        "atoms": 67,
        "kinetic_temperature_K": 16.037541705156464,
        "relative_error": 65.0,
        "step": 400,
        "thermostat_temperature_K": 1058.4777525403267
      }
    ]
  },
  "proposed_assessment": "violated"
}
```

### after

Input SHA-256: `83d8e7694f76c2ebe3b106953139e128f0813c4c2b8b437ca515e4a496e03ba6`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/gcmc-instrumented/after/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": {
    "atom_count_changed": true,
    "max_relative_error": 2.220446049250313e-16,
    "samples": [
      {
        "atoms": 2,
        "kinetic_temperature_K": 77.0,
        "relative_error": 0.0,
        "step": 0,
        "thermostat_temperature_K": 77.0
      },
      {
        "atoms": 38,
        "kinetic_temperature_K": 75.79542137882234,
        "relative_error": 0.0,
        "step": 100,
        "thermostat_temperature_K": 75.79542137882234
      },
      {
        "atoms": 38,
        "kinetic_temperature_K": 94.97063472747048,
        "relative_error": 0.0,
        "step": 200,
        "thermostat_temperature_K": 94.97063472747048
      },
      {
        "atoms": 67,
        "kinetic_temperature_K": 444.72470903298193,
        "relative_error": 2.220446049250313e-16,
        "step": 300,
        "thermostat_temperature_K": 444.724709032982
      },
      {
        "atoms": 67,
        "kinetic_temperature_K": 90.4259100307539,
        "relative_error": 0.0,
        "step": 400,
        "thermostat_temperature_K": 90.4259100307539
      }
    ]
  },
  "proposed_assessment": "satisfied"
}
```

Dependencies and limitations:

- Historical executable unavailable.
- GCMC trajectory and temperature behavior need independent verification.

## velocity

New one-atom reconstruction with an externally stated box-unit speed. The legacy command alone does not state the intended units; no detector is assumed.

Intent: Assign the kinetic energy computed for a tungsten primary knock-on atom.

Source: [m56524](https://matsci.org/t/inconsisten-kinetic-energy-calculation/56524). Provenance: `model_reconstruction`.

Source evidence:

- [Post 2](https://matsci.org/t/inconsisten-kinetic-energy-calculation/56524/2): correction. units box
- [Post 3](https://matsci.org/t/inconsisten-kinetic-energy-calculation/56524/3): confirmation. misread

The reconstruction adds `units box` to the velocity command.

### before

Input SHA-256: `0c0dcdda7d9f01a48c7b0da84fb0553cd3bcd3e91ba7e82b5cc0ac22db822911`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/velocity/before/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": {
    "intended_energy_eV": 95.268360648,
    "kinetic_energy_eV": 935.7228168756392,
    "relative_error": 8.821968285283841
  },
  "proposed_assessment": "violated"
}
```

### after

Input SHA-256: `7b34c6b723b71adf5c94a5467bdb73b34efc40286da405f42d19c263997d1781`.

[Native stdout](https://github.com/lan496/lamlet/blob/main/docs/assets/defect-corpus/2026-11-09/velocity/after/engine.stdout).

```json
{
  "engine_exit_code": 0,
  "timed_out": false,
  "measurement": {
    "intended_energy_eV": 95.268360648,
    "kinetic_energy_eV": 95.268360648,
    "relative_error": 0.0
  },
  "proposed_assessment": "satisfied"
}
```

Dependencies and limitations:

- Bonny_WHHe.txt is not attached.
- Original Python and potential dependent workflow has not been reproduced.
- A reduced one-atom calculation would be a reconstruction.

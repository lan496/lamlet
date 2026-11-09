import tempfile
from pathlib import Path
import unittest

from constraint_validation import measure_trajectory


def frame(step=0, second="2 1 0 0 0 0 0"):
    return (
        f"ITEM: TIMESTEP\n{step}\nITEM: NUMBER OF ATOMS\n2\n"
        "ITEM: BOX BOUNDS pp pp pp\n-5 5\n-5 5\n-5 5\n"
        f"ITEM: ATOMS id xu yu zu vx vy vz\n1 0 0 0 0 0 0\n{second}\n"
    )


class ConstraintMeasurements(unittest.TestCase):
    def measure(self, contents, steps=0):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "dump"
            path.write_text(contents)
            return measure_trajectory(path, steps, 1.0)

    def test_distance_and_radial_velocity_are_independent(self):
        self.assertEqual(self.measure(frame(second="2 1.25 0 0 0.5 4 0")), {
            "frames": 1, "max_relative_bond_error": 0.25,
            "max_radial_relative_velocity": 0.5,
        })
        self.assertEqual(self.measure(frame(second="2 1 0 0 0 4 0")), {
            "frames": 1, "max_relative_bond_error": 0.0,
            "max_radial_relative_velocity": 0.0,
        })

    def test_complete_trajectory_and_ids_are_required(self):
        for contents, steps in [(frame(), 1), (frame(1), 0),
                                (frame() + frame(1), 0),
                                (frame(second="1 1 0 0 0 0 0"), 0)]:
            with self.subTest(contents=contents, steps=steps):
                with self.assertRaises(ValueError):
                    self.measure(contents, steps)

    def test_nonfinite_and_malformed_frames_are_unassessed(self):
        for row in ["2 nan 0 0 0 0 0", "2 1 0 0 inf 0 0",
                    "2 0 0 0 0 0 0", "2 1 0 0 0 0"]:
            with self.subTest(row=row):
                with self.assertRaises(ValueError):
                    self.measure(frame(second=row))

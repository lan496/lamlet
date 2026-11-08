import unittest

from lammps_docs import page_record


class ManualCatalogTests(unittest.TestCase):
    def test_index_preserves_aliases_and_section_locations_without_promoting_prose(self):
        source = b'.. index:: fix nvt\n.. index:: fix npt\n\nfix nvt command\n===============\n\nSyntax\n""""""\n\nfix ID group nvt\n\nRestrictions\n""""""""""""\n\nThe axis must be periodic.\n'
        record = page_record("fix_nh.rst", source)
        self.assertEqual(record["commands"], ["fix npt", "fix nvt"])
        restriction = next(s for s in record["sections"] if s["title"] == "Restrictions")
        self.assertEqual(restriction["start_line"], 12)
        self.assertEqual(record["normative_signals"], 1)
        self.assertNotIn("rules", record)
        self.assertEqual(record["url"], "https://docs.lammps.org/fix_nh.html")


if __name__ == "__main__":
    unittest.main()

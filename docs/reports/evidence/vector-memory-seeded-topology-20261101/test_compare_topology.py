import tempfile, unittest
from pathlib import Path
from compare_topology import compare

class TopologyTests(unittest.TestCase):
    def setUp(self):
        self.dir=tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.a=Path(self.dir.name)/'a'; self.b=Path(self.dir.name)/'b'
        self.a.write_text('0 0 0 |0 1:0:1:3f800000\n1 0 1 |0 0:0:0:3f800000\n')
    def test_identical(self):
        self.b.write_bytes(self.a.read_bytes())
        self.assertTrue(compare(self.a,self.b)['identical_bytes'])
    def test_level_and_edge_change(self):
        self.b.write_text('0 1 0 |0 1:0:1:3f800000\n1 0 1 |0\n')
        result=compare(self.a,self.b)
        self.assertEqual(result['level_different_nodes'],1)
        self.assertEqual(result['neighbor_id_set_different_nodes'],1)
    def test_distance_change_is_not_edge_change(self):
        self.b.write_text('0 0 0 |0 1:0:1:3f800001\n1 0 1 |0 0:0:0:3f800000\n')
        result=compare(self.a,self.b)
        self.assertEqual(result['ordered_neighbor_records_different_nodes'],1)
        self.assertEqual(result['neighbor_id_set_different_nodes'],0)
    def test_duplicate_fails_closed(self):
        self.b.write_text('0 0 0 |0\n0 0 1 |0\n')
        with self.assertRaises(ValueError): compare(self.a,self.b)

if __name__=='__main__': unittest.main()

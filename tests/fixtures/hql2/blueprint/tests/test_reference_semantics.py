from __future__ import annotations
import copy
import json
import random
from pathlib import Path
import unittest
from reference_oracle import *

ROOT = Path(__file__).resolve().parents[1]
ROWS = [
 {'id':'A','row_key':'A','language':'en','vector':[0.1,0.0]},
 {'id':'B','row_key':'B','language':'th','vector':[0.2,0.0]},
 {'id':'C','row_key':'C','language':'th','vector':[0.3,0.0]},
]

class SemanticOracleTests(unittest.TestCase):
    def test_filter_topk_noncommutativity(self):
        post=[r['id'] for r in exact_topk(ROWS,[0,0],2) if r['language']=='th']
        pre=[r['id'] for r in exact_topk([r for r in ROWS if r['language']=='th'],[0,0],2)]
        self.assertEqual(post,['B']); self.assertEqual(pre,['B','C'])

    def test_candidate_exact_rerank_not_global_exact(self):
        candidates=[ROWS[1],ROWS[2]]
        self.assertEqual(exact_topk(candidates,[0,0],1)[0]['id'],'B')
        self.assertEqual(exact_topk(ROWS,[0,0],1)[0]['id'],'A')

    def test_topk_preserves_bag_rows(self):
        duplicate={**ROWS[0],'row_key':'A_path2'}
        self.assertEqual([r['id'] for r in exact_topk([ROWS[0],duplicate,ROWS[1]],[0,0],2)],['A','A'])

    def test_tie_order_stable(self):
        rows=[{'id':x,'row_key':x,'vector':[1,0]} for x in ['Z','A','M']]
        self.assertEqual([r['id'] for r in exact_topk(rows,[0,0],3)],['A','M','Z'])

    def test_zero_k(self):
        self.assertEqual(exact_topk(ROWS,[0,0],0),[])

    def test_dimension_rejected(self):
        with self.assertRaises(ValueError):distance([1],[1,2])

    def test_nonfinite_rejected(self):
        with self.assertRaises(ValueError):distance([float('nan')],[0])

    def test_cosine_zero_rejected_on_empty_input(self):
        with self.assertRaises(ValueError):exact_topk([], [0,0], 0, 'cosine')

    def test_distances(self):
        self.assertEqual(distance([1,2],[3,4]),8)
        self.assertAlmostEqual(distance([1,0],[0,1],'cosine'),1)
        self.assertEqual(distance([1,2],[3,4],'neg_dot'),-11)

    def test_three_valued_logic(self):
        self.assertIs(sql_and(True,None),None)
        self.assertIs(sql_and(False,None),False)
        self.assertIs(sql_or(True,None),True)
        self.assertIs(sql_or(False,None),None)

    def test_half_open_visibility(self):
        r={'tx_from':10,'tx_to':20,'valid_from':100,'valid_to':200}
        self.assertTrue(visible(r,10,100));self.assertFalse(visible(r,20,100))
        self.assertFalse(visible(r,10,200));self.assertFalse(visible(r,9,150))

    def test_timezone_normalization(self):
        self.assertEqual(instant_us('2026-01-01T07:00:00+07:00'),instant_us('2026-01-01T00:00:00Z'))
        with self.assertRaises(ValueError):instant_us('2026-01-01T00:00:00')

    def test_interval_correction_preserves_old_view(self):
        before={'value':'old','tx_from':1,'tx_to':5,'valid_from':0,'valid_to':100}
        fragments=[{'value':v,'tx_from':5,'tx_to':None,'valid_from':a,'valid_to':b}
                   for a,b,v in [(0,30,'old'),(30,50,'corrected'),(50,100,'old')]]
        records=[before]+fragments
        self.assertEqual([r['value'] for r in records if visible(r,4,40)],['old'])
        self.assertEqual([r['value'] for r in records if visible(r,5,40)],['corrected'])
        self.assertEqual([r['value'] for r in records if visible(r,5,50)],['old'])

    def test_seed_endpoint_predicates_differ(self):
        paths=[{'seed_language':'en','endpoint_language':'th'}, {'seed_language':'th','endpoint_language':'en'}]
        a=[p for p in paths if p['endpoint_language']=='th']
        b=[p for p in paths if p['seed_language']=='th']
        self.assertNotEqual(a,b)

    def test_optional_postfilter_removes_null_extension(self):
        left=[{'id':'A','a':None}]
        filtered=[r for r in left if r['a'] is not None and r['a']=='approved']
        self.assertEqual(filtered,[]);self.assertEqual(len(left),1)

    def test_i64_index_order(self):
        values=[-(1<<63),-100,-1,0,1,100,(1<<63)-1]
        self.assertEqual(sorted(values,key=key_i64),values)
        with self.assertRaises(ValueError):key_i64(1<<63)

    def test_f64_index_order(self):
        rng=random.Random(71); values=[rng.uniform(-1e6,1e6) for _ in range(1000)]
        self.assertEqual(sorted(values,key=key_f64),sorted(values))
        self.assertEqual(key_f64(-0.0),key_f64(0.0))

    def test_utf8_key_prefix_nul_order(self):
        values=['','\0','a','a\0','a\0a','aa','b','ไทย','🙂']
        self.assertEqual(sorted(values,key=key_utf8),sorted(values))

    def test_dag_validation(self):
        plan=json.loads((ROOT/'examples/query-ir.json').read_text())
        validate_dag(plan)
        broken=copy.deepcopy(plan);broken['nodes'][-1]['inputs']=['project']
        with self.assertRaises(ValueError):validate_dag(broken)

    def test_frozen_annotation(self):
        ann=json.loads((ROOT/'examples/annotation.json').read_text())
        ref=ann['targets'][0]['ref']; rev=ref['revision']
        record={'database_id':ref['database_id'],'id':'doc:one','namespace':'kb'}
        check_annotation_target(ann,{rev:record})
        with self.assertRaises(ValueError):check_annotation_target(ann,{rev:{**record,'id':'doc:two'}})
        without_database=copy.deepcopy(ann);del without_database['targets'][0]['ref']['database_id']
        with self.assertRaises(ValueError):check_annotation_target(without_database,{rev:record})

    def test_frozen_annotation_evidence(self):
        ann=json.loads((ROOT/'examples/annotation.json').read_text())
        ref=ann['targets'][0]['ref']; rev=ref['revision']
        record={'database_id':ref['database_id'],'id':'doc:one','namespace':'kb'}
        ann['evidence'][0]['ref']['revision']='00000000-0000-4000-8000-000000000002'
        with self.assertRaises(ValueError):check_annotation_target(ann,{rev:record})

    def test_cross_namespace_annotation(self):
        ann=json.loads((ROOT/'examples/annotation.json').read_text())
        ann['targets'][0]['ref']['namespace']='other'
        with self.assertRaises(ValueError):check_annotation_target(ann,{})

    def test_unicode_selector_units(self):
        text='ก้🙂x'
        self.assertEqual(text[2:3],'🙂')
        self.assertNotEqual(len(text),len(text.encode('utf-8')))
        self.assertNotEqual(len(text),len(text.encode('utf-16-le'))//2)

    def test_annotation_permission_intersection(self):
        self.assertFalse(authorized_annotation('ann1','doc1',{'doc1'}))
        self.assertFalse(authorized_annotation('ann1','doc1',{'ann1'}))
        self.assertTrue(authorized_annotation('ann1','doc1',{'ann1','doc1'}))

    def test_historical_versions_not_current_scan(self):
        old={'tx_from':1,'tx_to':3,'valid_from':0,'valid_to':None}
        self.assertTrue(visible(old,2,1));self.assertFalse(visible(old,3,1))

    def test_index_delta_merge_keeps_new_revision(self):
        indexed={'A_old'};delta={'A_new','B_new'};visible_ids={'A_new','B_new'}
        self.assertEqual((indexed|delta)&visible_ids,{'A_new','B_new'})

if __name__=='__main__':unittest.main()

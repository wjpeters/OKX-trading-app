import unittest,tempfile,pickle
from pathlib import Path
import numpy as np
import worker
class TrainingTests(unittest.TestCase):
 def test_fixed_walk_forward_never_enters_holdout(self):
  ds={'start_ms':0,'end_ms':365*worker.DAY,'holdout_start_ms':305*worker.DAY}
  wins=worker.split_windows(ds)
  self.assertEqual(len(wins),4)
  for w in wins:self.assertEqual(w['train'][1]-w['train'][0],180*worker.DAY);self.assertEqual(w['validation'][1]-w['validation'][0],30*worker.DAY);self.assertEqual(w['train'][1],w['validation'][0]);self.assertLessEqual(w['validation'][1],ds['holdout_start_ms'])
 def test_search_bounds_exclude_account_limits(self):
  low=worker.parameter_vector([0]*340+[-999]*12);high=worker.parameter_vector([0]*340+[999]*12)
  for name,lo,hi,_ in worker.PARAMETERS:self.assertGreaterEqual(low[name],lo);self.assertLessEqual(high[name],hi)
  self.assertNotIn('capital',low);self.assertNotIn('allocation',high)
 def test_holm_step_down_corrects_family(self):
  self.assertEqual(worker.holm([.001,.02,.04]),[True,True,True]);self.assertEqual(worker.holm([.03,.04,.001]),[False,False,True]);self.assertEqual(worker.holm([.02,.03,.04]),[False,False,False])
 def test_uncertain_and_negative_results_rejected(self):
  self.assertEqual(worker.bootstrap_p([{'return':.01}]*10,17),(1.,None));p,ci=worker.bootstrap_p([{'return':-.01}]*60,17,100);self.assertGreater(p,.9);self.assertLess(ci[1],0)
 def test_cma_checkpoint_rng_replays_next_population(self):
  es=worker.cma.CMAEvolutionStrategy([0.]*12,.3,{'seed':17,'popsize':8,'verb_disp':0,'verb_log':0})
  xs=es.ask();es.tell(xs,[sum(np.asarray(x)**2)for x in xs]);payload=pickle.dumps({'es':es,'rng':np.random.get_state()});expected=es.ask();saved=pickle.loads(payload);np.random.set_state(saved['rng']);actual=saved['es'].ask();np.testing.assert_array_equal(expected,actual)
 def test_atomic_checkpoint_preserves_complete_record(self):
  with tempfile.TemporaryDirectory()as tmp:
   p=Path(tmp)/'cp';worker.checkpoint(p,{'seed_index':1,'generations':23});self.assertEqual(pickle.loads(p.read_bytes())['generations'],23)
if __name__=='__main__':unittest.main()

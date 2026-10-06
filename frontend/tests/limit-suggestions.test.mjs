import test from 'node:test';
import assert from 'node:assert/strict';
import {suggestedLimits,emptyLimitDraft,changeLimit,applyLimitSuggestions,draftFromSavedLimits} from '../src/limit-suggestions.ts';

test('EUR 500 produces a complete, coherent example profile',()=>{
  assert.deepEqual(suggestedLimits('500'),{max_order:'50',max_position:'125',trade_risk:'5',daily_loss:'15',total_loss:'50',drawdown:'50',max_orders:'3',orders_per_minute:'2',spread_bps:'20'});
});
test('money ratios preserve decimal precision, including cents',()=>{
  assert.equal(suggestedLimits('0.01').trade_risk,'0.0001');
  assert.equal(suggestedLimits('0.1').max_order,'0.01');
  assert.equal(suggestedLimits('49.95').daily_loss,'1.4985');
  assert.equal(suggestedLimits('9999999999999999.12345678').trade_risk,'99999999999999.9912345678');
});
test('a budget change updates automatic fields and preserves personal overrides',()=>{
  let draft=changeLimit(emptyLimitDraft(),'capital','500');
  draft=changeLimit(draft,'trade_risk','3');draft=changeLimit(draft,'max_orders','1');
  draft=changeLimit(draft,'capital','1000');
  assert.equal(draft.values.max_order,'100');assert.equal(draft.values.max_position,'250');
  assert.equal(draft.values.trade_risk,'3');assert.equal(draft.values.max_orders,'1');
});
test('cleared or invalid budgets cannot leave stale automatic amounts',()=>{
  let draft=changeLimit(emptyLimitDraft(),'capital','500');draft=changeLimit(draft,'trade_risk','2');
  for(const value of ['', '0', '-500', 'NaN', 'Infinity', '1e3', '500xyz']) {
    const changed=changeLimit(draft,'capital',value);
    assert.equal(changed.values.max_order,'');assert.equal(changed.values.trade_risk,'2');
    assert.equal(suggestedLimits(value),null);
  }
});
test('explicit profile reset replaces overrides only in the unsaved draft',()=>{
  let draft=changeLimit(emptyLimitDraft(),'capital','500');draft=changeLimit(draft,'trade_risk','3');
  const applied=applyLimitSuggestions(draft);
  assert.equal(applied.values.trade_risk,'5');assert.deepEqual(applied.manual,{});
  assert.equal(draft.values.trade_risk,'3');
});
test('saved account limits remain personal choices on budget changes',()=>{
  const draft=draftFromSavedLimits({capital:'500',trade_risk:'2',max_order:'30',max_orders:1,account_uid:'fixture',confirmed:true});
  const changed=changeLimit(draft,'capital','1000');
  assert.equal(changed.values.trade_risk,'2');assert.equal(changed.values.max_order,'30');
  assert.equal(changed.values.max_orders,'1');assert.equal(changed.values.daily_loss,'30');
  assert.equal(changed.values.account_uid,undefined);assert.equal(changed.values.confirmed,undefined);
});
test('explicitly clearing an override preserves the cleared field for validation',()=>{
  let draft=changeLimit(emptyLimitDraft(),'capital','500');draft=changeLimit(draft,'trade_risk','');
  draft=changeLimit(draft,'capital','1000');assert.equal(draft.values.trade_risk,'');
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {chartPriceAtTick,acceptsChartLimitPrice} from '../src/chart-price.ts';

test('chart heights produce prices on the exact exchange tick',()=>{
  assert.equal(chartPriceAtTick(123.456,'0.01'),'123.46');
  assert.equal(chartPriceAtTick(123.454,'0.01'),'123.45');
  assert.equal(chartPriceAtTick(1.025,'0.05'),'1.05');
  assert.equal(chartPriceAtTick(98.75,'2.5'),'100.0');
  assert.equal(chartPriceAtTick(123.6,'1'),'124');
});
test('sub-cent and exponent coordinates preserve tick precision',()=>{
  assert.equal(chartPriceAtTick(0.000000126,'0.00000001'),'0.00000013');
  assert.equal(chartPriceAtTick(1e-8,'0.00000001'),'0.00000001');
  assert.equal(chartPriceAtTick(0.3,'0.1'),'0.3');
});
test('invalid and non-positive prices or ticks never populate a draft',()=>{
  for(const p of [NaN,Infinity,-Infinity,0,-10])assert.equal(chartPriceAtTick(p,'0.01'),null);
  for(const t of ['','0','-1','NaN','1e-2','0.01oops'])assert.equal(chartPriceAtTick(123,t),null);
  assert.equal(chartPriceAtTick(0.001,'0.01'),null);
});
test('chart picks target existing limit prices and preserve market/trigger meaning',()=>{
  for(const kind of ['limit','post_only','ioc','fok'])assert.equal(acceptsChartLimitPrice(kind,'market'),true);
  for(const execution of ['limit','ioc','fok'])assert.equal(acceptsChartLimitPrice('trigger',execution),true);
  for(const kind of ['market','tp_sl','trailing','iceberg','twap'])assert.equal(acceptsChartLimitPrice(kind,'limit'),false);
  assert.equal(acceptsChartLimitPrice('trigger','market'),false);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { buildPlan } from '../scripts/workflow-w1-plan.mjs';
const qbytes = readFileSync(new URL('../queue/QUERY_IR_W1_IMPLEMENTATION_QUEUE.json',import.meta.url));
const gbytes = readFileSync(new URL('../queue/QUERY_IR_W1_PROJECT_GRAPH.json',import.meta.url));

test('W1 overlay preserves history, requires materialization, and keeps dispatch disabled', () => {
  const plan = buildPlan(qbytes,gbytes);
  assert.equal(plan.dispatchAuthorized,false);
  assert.ok(plan.tasks.every(t => !t.dispatchAuthorized));
  assert.equal(plan.acceptance.length,8);
  assert.ok(plan.materializationBlocks.length > 0);
  assert.deepEqual(plan,JSON.parse(readFileSync(new URL('../queue/QUERY_IR_W1_EFFECTIVE_PLAN.json',import.meta.url))));
  assert.ok(plan.parallelWaves.some(w => ['H1','R1','X1'].every(id => w.includes(id))));
  const order = plan.parallelWaves.flat();
  for (const [a,b] of [['K1','S1'],['D1','I1'],['I1','V'],['V','RV'],['RV','A1'],['A1','FG']]) assert.ok(order.indexOf(a)<order.indexOf(b));
  assert.equal(plan.tasks.find(t => t.id==='RV').execution.model,'gpt-6-luna');
});
test('duplicate known node cannot hide a missing graph node', () => {
  const graph=JSON.parse(gbytes); graph.nodes[1]=structuredClone(graph.nodes[0]);
  assert.throws(() => buildPlan(qbytes,Buffer.from(JSON.stringify(graph))),/unique and exactly equal/);
});
test('source identity is stable across Git LF and CRLF checkouts', () => {
  const lf = value => Buffer.from(value.toString('utf8').replace(/\r\n/g,'\n'));
  const crlf = value => Buffer.from(lf(value).toString('utf8').replace(/\n/g,'\r\n'));
  assert.deepEqual(buildPlan(lf(qbytes),lf(gbytes)),buildPlan(crlf(qbytes),crlf(gbytes)));
});
test('source mismatch and dispatch mutation fail closed', () => {
  for (const change of [g => { g.source_sha='bad'; },g => { g.dispatch_authorized=true; },g => { g.nodes[0].write_set=[]; }]) {
    const graph=JSON.parse(gbytes); change(graph);
    assert.throws(() => buildPlan(qbytes,Buffer.from(JSON.stringify(graph))),/mismatch/i);
  }
});

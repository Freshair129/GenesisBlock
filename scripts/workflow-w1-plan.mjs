// Planning adapter only: deliberately not an executable guard plan.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const queuePath = 'queue/QUERY_IR_W1_IMPLEMENTATION_QUEUE.json';
const graphPath = 'queue/QUERY_IR_W1_PROJECT_GRAPH.json';
const outputPath = 'queue/QUERY_IR_W1_EFFECTIVE_PLAN.json';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
// Git text checkout may use CRLF on Windows. Bind LF-normalized UTF-8 source
// bytes so the same tracked snapshot has one identity on both CI platforms.
const sourceDigest = bytes => sha256(Buffer.from(bytes.toString('utf8').replace(/\r\n/g, '\n'), 'utf8'));

export function buildPlan(queueBytes, graphBytes) {
  const queue = JSON.parse(queueBytes), graph = JSON.parse(graphBytes);
  const ids = queue.tasks.map(t => t.id), nodes = graph.nodes.map(n => n.id);
  const sorted = values => [...values].sort();
  const same = (a, b) => JSON.stringify(sorted(a)) === JSON.stringify(sorted(b));
  if (new Set(ids).size !== ids.length || new Set(nodes).size !== nodes.length || !same(ids, nodes)) {
    throw new Error('Source queue/graph IDs must be unique and exactly equal');
  }
  if (queue.source_sha !== graph.source_sha || queue.dispatch_authorized !== false || graph.dispatch_authorized !== false) {
    throw new Error('Historical source identity/dispatch boundary mismatch');
  }
  for (const task of queue.tasks) {
    const node = graph.nodes.find(n => n.id === task.id);
    if (!same(task.deps, node.depends_on) || !same(task.write_set, node.write_set)) throw new Error(`Source mismatch: ${task.id}`);
  }
  const declaredEdges = queue.tasks.flatMap(t => t.deps.map(d => JSON.stringify([d,t.id])));
  const actualEdges = graph.edges.map(e => JSON.stringify([e.from,e.to]));
  if (!same(declaredEdges,actualEdges)) throw new Error('Source graph edges mismatch');
  const sol = role => ({ role, model: 'gpt-6.1-sol', reasoningEffort: 'high' });
  const luna = role => ({ role, model: 'gpt-6-luna', reasoningEffort: 'max' });
  const roles = { P0: sol('orchestrator'), G0: sol('decision'), D0: sol('decision'), X1: sol('audit'), V: luna('verify'), RV: luna('review'), FG: sol('orchestrator') };
  const tasks = queue.tasks.map(t => ({
    id: t.id, title: t.title, deps: [...t.deps], writePaths: [...t.write_set],
    execution: roles[t.id] ?? luna('worker'), dispatchAuthorized: false,
    verifyCommandsToMaterialize: [...t.verify], acceptance: t.exit
  }));
  const get = id => tasks.find(t => t.id === id);
  tasks.push({ id: 'K1', title: 'Resolve normative contracts and enumerate case-level AC', deps: ['D0'], writePaths: [], execution: sol('decision'), dispatchAuthorized: false,
    acceptance: 'Pin each bound/default/error mapping and expected outcome before S1; every case has a required flag and exact command identity.' });
  get('S1').deps = ['K1'];
  get('F1').acceptance = 'Fixture containers validate; positive requests are schema-valid, schema-invalid requests are rejected, and valid-but-unsupported requests yield the specified typed error. Never require every negative request to validate.';
  get('F1').reopen = 'A normative mismatch returns to K1 and a new approved plan revision; revalidate S1 and every dependent task.';
  get('C0').kind = 'characterization';
  get('C0').acceptance = 'Record every case as expected GREEN or a specifically approved expected RED; RED acceptance is characterization only, never an integrated GREEN result.';
  get('H1').title = 'Test HQL compatibility and route production findings to the source owner';
  get('H1').correctionOwner = { 'src/lib.rs': 'C1', 'src/query/ast.rs': 'new Sol Decision scope required', 'src/query/hql.pest': 'new Sol Decision scope required' };
  get('R1').fixtureOwner = 'F1; reopening creates a revised plan and invalidates dependent evidence; R1 cannot edit shared fixtures';
  get('X1').deps = ['C1'];
  // All document edits precede candidate freeze, preventing status -> new SHA -> reverify loops.
  get('D1').deps = ['N1', 'X1'];
  get('D1').acceptance = 'Freeze normative docs, scope and expected evidence inventory before integration; record final PASS and run receipts outside the candidate tree.';
  tasks.push({ id: 'I1', title: 'Assemble immutable candidate and integration provenance', deps: ['D1'], writePaths: [], execution: sol('integration'), dispatchAuthorized: false,
    acceptance: 'Record ordered task commits, tree/source digests and atomic C0+C1 GREEN closure; no unverified generated or documentation changes remain.' });
  get('V').deps = ['I1'];
  get('RV').deps = ['V'];
  get('RV').acceptance = 'Independent Luna Max Review on the exact integrated candidate, after Verify PASS; no author or integrator self-review.';
  tasks.push({ id: 'A1', title: 'Independent final scope and provenance audit', deps: ['RV'], writePaths: [], execution: sol('audit'), dispatchAuthorized: false,
    acceptance: 'Separate Sol session checks effective AC, source and artifact hashes, all receipts, scope and remaining NOT_RUN items; consumer audit X1 cannot substitute.' });
  get('FG').deps = ['A1'];
  get('FG').acceptance = 'Fresh independent Verify, Review, final Audit and integration receipts bind one candidate; all required AC pass and no blocking findings remain. Completion does not authorize merge or release.';
  const effectiveAc = queue.acceptance_criteria.map((criterion, index) => ({
    id: `W1-AC-${String(index + 1).padStart(2, '0')}`, required: true,
    criterion: index === 6 ? 'Independent Luna Max Review and separate Sol final provenance/scope Audit pass on the integrated candidate; canonical docs and external receipts identify evidence and remaining NOT_RUN items.' : criterion,
    tasks: [['K1','S1','F1'],['C0','C1','H1','R1','N1'],['F1','C0','R1','N1'],['K1','F1'],['H1'],['I1','V'],['RV','A1','FG'],['X1','D1','A1']][index]
  }));
  const plan = {
    schemaVersion: 1, artifactType: 'planning-overlay', id: 'QUERY-IR-W1-EFFECTIVE-20261010', dispatchAuthorized: false,
    authorizationScope: 'Guard hardening and synthetic pilot approved; W1 implementation remains disabled',
    sourceDigestEncoding: 'utf8-lf',
    sources: [ { path: queuePath, sha256: sourceDigest(queueBytes) }, { path: graphPath, sha256: sourceDigest(graphBytes) } ],
    historicalDeclaredSourceSha: queue.source_sha,
    supersedes: ['Historical model assignments only: user selects Luna Max execution/Verify/Review and Sol Decision/Integration/Audit/Orchestration', 'Task contradictions corrected by explicit K1/I1/A1 gates and owner/reopening rules'],
    tasks, graph: { nodes: tasks.map(t => t.id), edges: tasks.flatMap(t => t.deps.map(d => [d,t.id])) },
    acceptance: effectiveAc,
    atomicIntegrationGroups: [{ tasks: ['C0','C1'], requiredOutcome: 'green', neverMergeRedAlone: true }],
    materializationBlocks: [
      'Record owner W1 scope and dispatch authorization for the exact immutable execution plan',
      'K1 resolves per-case normative decisions and maps required AC to allowlisted executable/argv/cwd commands',
      'Pin baseline/candidate, concrete task and command deadlines, command budgets and all source/artifact identities',
      'Create isolated workspaces and external run ledger; register actual provider session receipts through the trusted coordinator',
      'Compile to SPEC--EXECUTION-DAG-GUARD-V1 closed schema and pass validatePlan before dispatch'
    ]
  };
  const visited = new Set(), active = new Set();
  function visit(id) {
    if (active.has(id)) throw new Error(`Cycle: ${id}`);
    if (visited.has(id)) return;
    const task = get(id);
    if (!task) throw new Error(`Unknown task: ${id}`);
    active.add(id); task.deps.forEach(visit); active.delete(id); visited.add(id);
  }
  tasks.forEach(t => visit(t.id));
  plan.parallelWaves = [];
  const remaining = new Set(tasks.map(t => t.id)), done = new Set();
  while (remaining.size) {
    const wave = tasks.filter(t => remaining.has(t.id) && t.deps.every(d => done.has(d))).map(t => t.id);
    plan.parallelWaves.push(wave); wave.forEach(id => { remaining.delete(id); done.add(id); });
  }
  return plan;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const bytes = JSON.stringify(buildPlan(readFileSync(resolve(root,queuePath)), readFileSync(resolve(root,graphPath))), null, 2) + '\n';
  if (process.argv.includes('--write')) writeFileSync(resolve(root,outputPath), bytes);
  else if (readFileSync(resolve(root,outputPath),'utf8').replace(/\r\n/g,'\n') !== bytes) throw new Error('Effective W1 plan is stale; inspect changes then regenerate with --write');
  console.log(JSON.stringify({ ok:true, dispatchAuthorized:false, artifact:outputPath }));
}

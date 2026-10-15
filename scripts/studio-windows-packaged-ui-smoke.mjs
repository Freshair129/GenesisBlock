import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const requireFromStudio = createRequire(new URL('../studio/package.json', import.meta.url));
const { chromium } = requireFromStudio('@playwright/test');

const cdpPort = Number(process.env.STUDIO_CDP_PORT);
const missingRoot = process.env.STUDIO_MISSING_ROOT;
const nonDirectoryPath = process.env.STUDIO_NON_DIRECTORY_PATH;
const fixtureRoot = process.env.STUDIO_FIXTURE_ROOT;
const parityToken = process.env.STUDIO_PARITY_TOKEN;
const evidenceDirectory = process.env.STUDIO_EVIDENCE_DIRECTORY;
const signals = {
  pickerCancelRequested: process.env.STUDIO_PICKER_CANCEL_REQUESTED_FILE,
  pickerCancelled: process.env.STUDIO_PICKER_CANCELLED_FILE,
  pickerSelectRequested: process.env.STUDIO_PICKER_SELECT_REQUESTED_FILE,
  pickerSelected: process.env.STUDIO_PICKER_SELECTED_FILE,
  localCaptured: process.env.STUDIO_LOCAL_CAPTURED_FILE,
  remoteReady: process.env.STUDIO_REMOTE_READY_FILE,
};

assert.ok(Number.isInteger(cdpPort) && cdpPort > 0, 'Studio CDP port must be set');
assert.ok(missingRoot, 'Studio missing-root fixture path must be set');
assert.ok(nonDirectoryPath, 'Studio non-directory fixture path must be set');
assert.ok(fixtureRoot && existsSync(fixtureRoot), 'Persisted Studio parity fixture must exist');
assert.ok(parityToken, 'Studio parity token must be set');
assert.ok(evidenceDirectory, 'Studio evidence directory must be set');
for (const [name, file] of Object.entries(signals)) assert.ok(file, `${name} signal path must be set`);
assert.equal(existsSync(missingRoot), false, 'Missing-root fixture unexpectedly exists');

writeFileSync(nonDirectoryPath, 'This fixture path is a file, not a data-root folder.');

const cdpUrl = `http://127.0.0.1:${cdpPort}`;
const browserDeadline = Date.now() + 60_000;
let browser;

async function waitForFile(file, timeoutMs, description) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(file)) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${description}`);
}

async function waitForPathError(page, expectedPath, expectedMessage) {
  await page.waitForFunction(
    ({ path: pathValue, message }) => {
      const text = document.querySelector('[role="alert"]')?.innerText ?? '';
      return text.includes(pathValue) && text.toLowerCase().includes(message.toLowerCase());
    },
    { path: expectedPath, message: expectedMessage },
    { timeout: 15_000 },
  );
}

async function readLocalNativeOperations(page) {
  return page.evaluate(async () => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== 'function') {
      throw new Error('Packaged Tauri invoke bridge was not available for parity evidence');
    }
    // The UI has already opened this exclusively owned root through Connect local.
    // Read capabilities from that session instead of opening the same root twice.
    const capabilities = await invoke('studio_local_capabilities');
    const status = await invoke('studio_local_status');
    const graph = await invoke('studio_local_graph', {
      request: { limit: 240, direction: 'both' },
    });
    const expansion = await invoke('studio_local_graph', {
      request: { seed: 'b', limit: capabilities.limits.expansion_nodes, direction: 'both' },
    });
    const collections = await invoke('studio_local_collections');
    const schemas = await invoke('studio_local_relational_schemas');
    const inspection = await invoke('studio_local_inspect', { entityId: 'b' });
    const hql = await invoke('studio_local_hql', { query: 'TRAVERSE FROM a DEPTH 1 REL LINKS' });
    const query = schemas[0]?.named_queries?.find((entry) => entry.name === 'list_projects');
    if (!query) throw new Error('Persisted parity fixture has no registered list_projects query');
    const namedQuery = await invoke('studio_local_named_query', {
      request: {
        namespace: schemas[0].namespace,
        schema_version: schemas[0].schema_version,
        query_name: query.name,
        parameters: {},
        limit: query.default_limit,
      },
    });
    return { capabilities, status, graph, expansion, collections, schemas, inspection, hql, namedQuery };
  });
}

async function readRemoteNativeOperations(page, expansionLimit) {
  return page.evaluate(async (limit) => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== 'function') {
      throw new Error('Packaged Tauri invoke bridge was not available for parity evidence');
    }
    const request = (operation) => invoke('studio_remote_request', { request: operation });
    const capabilities = await request({ kind: 'capabilities' });
    const [status, frontier, graph, expansion, collections, schemas, inspection, hql] = await Promise.all([
      request({ kind: 'status' }),
      request({ kind: 'frontier' }),
      request({ kind: 'graph', limit: 240, direction: 'both' }),
      request({ kind: 'graph', seed: 'b', limit, direction: 'both' }),
      request({ kind: 'collections' }),
      request({ kind: 'relationalSchemas' }),
      request({ kind: 'entity', entityId: 'b' }),
      request({ kind: 'readOnlyHql', query: 'TRAVERSE FROM a DEPTH 1 REL LINKS' }),
    ]);
    const query = schemas[0]?.named_queries?.find((entry) => entry.name === 'list_projects');
    if (!query) throw new Error('Persisted parity fixture has no registered list_projects query');
    const namedQuery = await request({
      kind: 'namedQuery',
      request: {
        namespace: schemas[0].namespace,
        schema_version: schemas[0].schema_version,
        query_name: query.name,
        parameters: {},
        limit: query.default_limit,
      },
    });
    return { capabilities, status, frontier, graph, expansion, collections, schemas, inspection, hql, namedQuery };
  }, expansionLimit);
}

async function connectLocal(page) {
  await page.getByRole('button', { name: 'Connect local' }).click();
  await page.waitForFunction(() => {
    const connection = document.querySelector('.connection-chip b')?.textContent ?? '';
    const error = document.querySelector('[role="alert"]')?.textContent?.trim() ?? '';
    return connection.includes('local / local') || error.length > 0;
  }, undefined, { timeout: 30_000 }).catch(async (error) => {
    const state = await page.evaluate(() => ({
      screen: document.querySelector('.connection-screen') ? 'connection' : 'workspace',
      connection: document.querySelector('.connection-chip b')?.textContent?.trim() ?? null,
      alerts: Array.from(document.querySelectorAll('[role="alert"]'))
        .map((alert) => alert.textContent?.trim() ?? '')
        .filter(Boolean),
      root: document.querySelector('#genesis-data-root')?.value ?? null,
    }));
    console.error(`Packaged local connection state did not settle: ${JSON.stringify(state)}`);
    throw error;
  });
  const connectionState = await page.evaluate(() => ({
    connection: document.querySelector('.connection-chip b')?.textContent?.trim() ?? null,
    alerts: Array.from(document.querySelectorAll('[role="alert"]'))
      .map((alert) => alert.textContent?.trim() ?? '')
      .filter(Boolean),
  }));
  assert.ok(
    connectionState.connection?.includes('local / local'),
    `Packaged local connect failed: ${JSON.stringify(connectionState)}`,
  );
  await page.locator('.metric-strip').waitFor({ state: 'visible', timeout: 30_000 });
  assert.deepEqual(connectionState.alerts, [], 'Packaged local connection displayed an error');
}

async function connectRemote(page, restBaseUrl) {
  await page.getByRole('button', { name: 'Remote server' }).click();
  await page.getByLabel('SERVER URL').fill(restBaseUrl);
  await page.getByLabel('BEARER TOKEN').fill(parityToken);
  await page.getByRole('button', { name: 'Connect remote' }).click();
  await page.waitForFunction(
    () => {
      const negotiatedState = document.querySelector('.connection-chip b')?.textContent ?? '';
      const visibleStatus = document.querySelector('.truth-badge')?.innerText ?? '';
      // The responsive layout hides the connection-chip label at the hosted
      // runner's 1024px width; verify negotiated state plus the visible badge.
      return negotiatedState.includes('remote / remote')
        && visibleStatus.includes('REMOTE / READ ONLY');
    },
    undefined,
    { timeout: 30_000 },
  );
  await page.locator('.metric-strip').waitFor({ state: 'visible', timeout: 30_000 });
  assert.equal(await page.getByRole('alert').count(), 0, 'Packaged remote connection displayed an error');
}

async function exerciseReadSurfaces(page) {
  await page.getByRole('button', { name: 'Overview workspace' }).click();
  await page.locator('.card-grid').waitFor({ state: 'visible' });
  const overview = await page.locator('.card-grid article').evaluateAll((articles) => (
    Object.fromEntries(articles.slice(0, 6).map((article) => [
      article.querySelector('span')?.textContent?.trim(),
      article.querySelector('strong')?.textContent?.trim(),
    ]))
  ));
  assert.equal(overview.Nodes, '3', 'Packaged overview did not read the three fixture nodes');
  assert.equal(overview.Edges, '2', 'Packaged overview did not read the two fixture edges');
  assert.equal(overview.Collections, '1', 'Packaged overview collection count differs from the fixture');

  await page.getByRole('button', { name: 'Vectors workspace' }).click();
  await page.getByText('default', { exact: true }).waitFor({ state: 'visible' });
  const vectorCollections = await page.locator('.card-grid article').evaluateAll((articles) => (
    articles.map((article) => article.innerText.trim())
  ));
  assert.equal(vectorCollections.length, 1, 'Packaged vector view did not show the fixture collection');
  assert.match(vectorCollections[0], /1 vectors/);

  await page.getByRole('button', { name: 'Data workspace' }).click();
  await page.getByText('studio_fixture', { exact: true }).waitFor({ state: 'visible' });
  const querySelect = page.locator('.relational-query select');
  await querySelect.selectOption('studio_fixture:list_projects');
  await page.getByRole('button', { name: 'Run named query' }).click();
  const namedQueryOutput = page.locator('.relational-query pre');
  await page.waitForFunction(
    () => document.querySelector('.relational-query pre')?.textContent?.includes('Studio fixture project'),
    undefined,
    { timeout: 20_000 },
  );
  const namedQuery = JSON.parse(await namedQueryOutput.innerText());

  await page.getByRole('button', { name: 'Query workspace' }).click();
  const queryEditor = page.locator('#hql-query-editor');
  await queryEditor.fill('TRAVERSE FROM a DEPTH 1 REL LINKS');
  await page.getByRole('button', { name: 'Run bounded query' }).click();
  await page.waitForFunction(
    () => document.querySelector('.query-workbench pre')?.textContent?.trim() !== 'Results appear here with no write fallback.',
    undefined,
    { timeout: 20_000 },
  );
  const hqlOutput = JSON.parse(await page.locator('.query-workbench pre').innerText());
  assert.ok(Array.isArray(hqlOutput) && hqlOutput.length > 0, 'Fixture HQL read returned no results');

  await page.getByRole('button', { name: 'Graph workspace' }).click();
  await page.getByText('Browse scene nodes (3)', { exact: true }).click();
  await page.getByRole('button', { name: /Bravo, knowledge, Agent/ }).click();
  await page.getByRole('heading', { name: 'Bravo', level: 2 }).waitFor({ state: 'visible' });
  const inspector = page.locator('.inspector').last();
  const inspectorText = await inspector.innerText();
  assert.match(inspectorText, /2 links/);

  return { overview, vectorCollections, namedQuery, hqlOutput, inspectorText };
}

function comparableCapabilities(value) {
  return {
    protocol_version: value.protocol_version,
    engine_version: value.engine_version,
    read_features: value.read_features,
    write_features: value.write_features,
    limits: value.limits,
    consistency: value.consistency,
  };
}

function comparableScene(value) {
  const { scene_id: _sceneId, ...scene } = value;
  return scene;
}

function summarizeParity(local, remote, visibleResults) {
  const localCapabilities = local.capabilities;
  const remoteCapabilities = remote.capabilities;
  assert.equal(localCapabilities.mode, 'local');
  assert.equal(remoteCapabilities.mode, 'remote');
  assert.deepEqual(comparableCapabilities(localCapabilities), comparableCapabilities(remoteCapabilities));
  assert.deepEqual(localCapabilities.write_features, [], 'Fixture local mode unexpectedly declares writes');

  const localStatus = local.status;
  const remoteStatus = remote.status;
  const localFrontier = localStatus.frontier;
  const remoteFrontier = remote.frontier;
  assert.deepEqual(
    {
      node_count: localStatus.node_count,
      edge_count: localStatus.edge_count,
      collection_count: localStatus.collection_count,
      index_lag: localStatus.index_lag,
      frontier: localFrontier,
    },
    {
      node_count: remoteStatus.node_count,
      edge_count: remoteStatus.edge_count,
      collection_count: Array.isArray(remoteStatus.collections) ? remoteStatus.collections.length : 0,
      index_lag: remoteStatus.index_lag,
      frontier: remoteFrontier.frame,
    },
    'Packaged local and remote status differ on the same persisted fixture',
  );
  assert.equal(localStatus.node_count, 3);
  assert.equal(localStatus.edge_count, 2);
  assert.equal(localStatus.collection_count, 1);

  const localScene = local.graph;
  const remoteScene = remote.graph;
  assert.deepEqual(comparableScene(localScene), comparableScene(remoteScene));
  assert.deepEqual(localScene.nodes.map((node) => node.id), ['a', 'b', 'c']);
  assert.equal(localScene.edges.length, 2);

  assert.deepEqual(comparableScene(local.expansion), comparableScene(remote.expansion));
  assert.deepEqual(local.expansion.nodes.map((node) => node.id), ['a', 'b', 'c']);
  assert.equal(local.expansion.edges.length, 2);

  const localCollections = local.collections;
  const remoteCollections = remote.collections;
  assert.deepEqual(localCollections, remoteCollections);

  const localSchemas = local.schemas;
  const remoteSchemas = remote.schemas;
  assert.deepEqual(localSchemas, remoteSchemas);
  assert.equal(localSchemas[0]?.namespace, 'studio_fixture');

  const localInspection = local.inspection;
  const remoteInspection = remote.inspection;
  assert.deepEqual(localInspection, remoteInspection);
  assert.equal(localInspection.entity_id, 'b');
  assert.equal(localInspection.vector_present, true, 'Packaged fixture vector was not visible in entity inspection');

  const localHql = local.hql;
  const remoteHql = remote.hql;
  assert.deepEqual(localHql, remoteHql);
  assert.ok(Array.isArray(localHql) && localHql.length > 0);

  const localNamedQuery = local.namedQuery;
  const remoteNamedQuery = remote.namedQuery;
  assert.deepEqual(localNamedQuery, remoteNamedQuery);
  assert.ok(JSON.stringify(localNamedQuery).includes('Studio fixture project'));

  const comparableVisibleResults = (results) => ({
    overview: Object.fromEntries(
      ['Nodes', 'Edges', 'Collections', 'Index lag'].map((label) => [label, results.overview[label]]),
    ),
    vectorCollections: results.vectorCollections,
    namedQuery: results.namedQuery,
    hqlOutput: results.hqlOutput,
    inspectorText: results.inspectorText,
  });
  assert.deepEqual(
    comparableVisibleResults(visibleResults.local),
    comparableVisibleResults(visibleResults.remote),
    'Packaged local and remote S1 views differ for the persisted fixture',
  );
  assert.ok(JSON.stringify(visibleResults).includes('Studio fixture project'));
  return {
    fixtureId: path.win32.basename(fixtureRoot),
    comparedOperations: [
      'read capabilities and limits',
      'status and frontier',
      'bounded graph scene',
      'seeded graph expansion',
      'collections',
      'relational schemas',
      'entity inspection',
      'read-only HQL',
      'registered named query',
    ],
    visibleFixtureResults: {
      nodeCount: visibleResults.overview.Nodes,
      edgeCount: visibleResults.overview.Edges,
      namedQueryRows: localNamedQuery.length,
      hqlRows: localHql.length,
      inspectedEntity: localInspection.entity_id,
    },
  };
}

try {
  let cdpReady = false;
  while (Date.now() < browserDeadline) {
    try {
      const response = await fetch(`${cdpUrl}/json/version`);
      if (response.ok) {
        cdpReady = true;
        break;
      }
    } catch {
      // The packaged WebView2 may still be starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  assert.ok(cdpReady, `Packaged WebView2 CDP endpoint did not become ready at ${cdpUrl}`);
  console.log(`Packaged WebView2 CDP endpoint ready at ${cdpUrl}.`);
  browser = await chromium.connectOverCDP(cdpUrl, { timeout: 15_000 });
  let page;
  const pageDeadline = Date.now() + 30_000;
  while (!page && Date.now() < pageDeadline) {
    for (const context of browser.contexts()) {
      for (const candidate of context.pages()) {
        if (await candidate.getByText('GENESIS / STUDIO', { exact: true }).count()) {
          page = candidate;
          break;
        }
      }
      if (page) break;
    }
    if (!page) await new Promise((resolve) => setTimeout(resolve, 250));
  }
  assert.ok(page, 'Packaged Studio connection screen was not visible over WebView2 CDP');
  const errorAlert = page.getByRole('alert');
  const rootInput = page.locator('#genesis-data-root');
  await rootInput.fill(missingRoot);
  await page.getByRole('button', { name: 'Connect local' }).click();
  await waitForPathError(page, missingRoot, 'could not be accessed');
  let errorText = await errorAlert.innerText();
  assert.match(errorText, /could not be accessed/i, 'Missing path did not reach packaged native validation');
  assert.ok(errorText.includes(missingRoot), 'Missing-path error did not identify the configured path');
  console.log('Packaged missing-root path handling passed.');

  await rootInput.fill(nonDirectoryPath);
  await page.getByRole('button', { name: 'Connect local' }).click();
  await waitForPathError(page, nonDirectoryPath, 'is not a folder');
  errorText = await errorAlert.innerText();
  assert.match(errorText, /is not a folder/i, 'Non-directory path did not reach packaged native validation');
  assert.ok(errorText.includes(nonDirectoryPath), 'Non-directory error did not identify the configured path');
  console.log('Packaged non-directory path handling passed.');

  await page.getByRole('button', { name: 'Browse', exact: true }).click();
  writeFileSync(signals.pickerCancelRequested, 'requested');
  await waitForFile(signals.pickerCancelled, 60_000, 'Windows UI Automation to cancel the native folder picker');
  assert.equal(await rootInput.inputValue(), nonDirectoryPath, 'Cancelling the picker changed the configured path');
  assert.equal(await errorAlert.count(), 0, 'Cancelling the picker left a stale error visible');
  console.log('Packaged native folder picker cancellation passed.');

  await page.getByRole('button', { name: 'Browse', exact: true }).click();
  writeFileSync(signals.pickerSelectRequested, 'requested');
  await waitForFile(signals.pickerSelected, 60_000, 'Windows UI Automation to select the persisted fixture folder');
  await page.waitForFunction(
    (expectedPath) => document.querySelector('#genesis-data-root')?.value === expectedPath,
    fixtureRoot,
    { timeout: 15_000 },
  );
  const selectedRoot = path.win32.resolve(await rootInput.inputValue()).toLowerCase();
  assert.equal(
    selectedRoot,
    path.win32.resolve(fixtureRoot).toLowerCase(),
    'Selecting the fixture in the native picker did not populate the exact data-root path',
  );
  await connectLocal(page);
  const localVisibleResults = await exerciseReadSurfaces(page);
  const localNativeResults = await readLocalNativeOperations(page);
  await page.getByRole('button', { name: 'Connections' }).click();
  await page.getByRole('button', { name: 'Connect local' }).waitFor({ state: 'visible', timeout: 15_000 });
  writeFileSync(signals.localCaptured, 'captured-and-disconnected');

  await waitForFile(signals.remoteReady, 90_000, 'the same-fixture authenticated REST server to start');
  const restBaseUrl = readFileSync(signals.remoteReady, 'utf8').trim();
  assert.match(restBaseUrl, /^http:\/\/127\.0\.0\.1:\d+$/, 'REST smoke server must use loopback HTTP');
  await connectRemote(page, restBaseUrl);
  const remoteVisibleResults = await exerciseReadSurfaces(page);
  const remoteNativeResults = await readRemoteNativeOperations(
    page,
    localNativeResults.capabilities.limits.expansion_nodes,
  );
  const summary = summarizeParity(localNativeResults, remoteNativeResults, {
    local: localVisibleResults,
    remote: remoteVisibleResults,
  });
  writeFileSync(
    path.join(evidenceDirectory, 'studio-packaged-parity.json'),
    `${JSON.stringify(summary, null, 2)}\n`,
    'utf8',
  );
  console.log(`Packaged local/remote parity passed for ${summary.comparedOperations.length} read operations.`);
} finally {
  if (browser) await browser.close();
}

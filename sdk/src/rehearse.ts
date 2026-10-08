// Disposable-node CCC acceptance. Public fixture key 0x46 repeated; no user wallet.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { ccc } from '@ckb-ccc/shell';
import { AgoraError, DAO, SIGHASH, decodeProposal, decodeStatement, pins, prepareCreation, prepareSettlement, prepareVote, verifyDeployment, type Deployment } from './client.ts';

const args = process.argv.slice(2);
const resume = args.length === 5 && args[4] === '--resume';
assert(args.length === 4 || resume, 'usage: rehearse.ts CKB_REPO CKB_BINARY COMPLETED_NATIVE_RUN SDK_DIRECTORY [--resume] (repository cwd)');
const [repo, bin, nativeRun, directory] = args.slice(0, 4).map(value => resolve(value));
const root = process.cwd();
if (!resume) await mkdir(directory); // Never overwrite an attempt or restart a foreign node.
const config = await readFile(join(nativeRun, 'ckb-node/ckb.toml'), 'utf8');
const port = config.match(/listen_address = "127\.0\.0\.1:(\d+)"/)?.[1];
assert(port, 'owned loopback configuration');
const url = 'http://127.0.0.1:' + port;
const serverPath = join(directory, 'owned-node-server');
await copyFile(join(root, 'target/debug/agoraseal-node-serve'), serverPath);
const server = spawn(serverPath, [repo, bin, nativeRun], { stdio: ['pipe', 'pipe', 'pipe'] });
const serverClosed = once(server, 'exit');
server.stdout.on('data', value => process.stdout.write(value));
server.stderr.on('data', value => process.stderr.write(value));
let rpcId = 0;
async function rpc<T>(method: string, params: unknown[] = []): Promise<T> {
  const response = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: ++rpcId, method, params }), signal: AbortSignal.timeout(30_000) });
  assert(response.ok, 'local RPC HTTP error');
  const data = await response.json() as { error?: unknown; result: T };
  if (data.error) throw new Error(method + ': ' + JSON.stringify(data.error));
  return data.result;
}
async function tool(executable: string, toolArgs: string[]): Promise<void> {
  const child = spawn(join(root, executable), toolArgs, { cwd: root, stdio: 'inherit' });
  const [code] = await once(child, 'exit'); assert.equal(code, 0, executable + ' failed');
}
async function mining<T>(work: Promise<T>): Promise<T> {
  let done = false; const result = work.finally(() => { done = true; });
  // Attach rejection immediately while the local miner is awaiting RPC.
  void result.catch(() => {});
  try {
    for (let count = 0; !done && count < 100; count++) { await delay(400); if (!done) await rpc('generate_block'); }
    assert(done, 'bounded local confirmation timeout'); return await result;
  } catch (error) { void result.catch(() => {}); throw error; }
}
const rows: unknown[] = [];
let clientOwner: ReturnType<typeof ccc.ClientPublicTestnet.open> | undefined;
try {
  let connected = false;
  for (let retry = 0; retry < 100; retry++) {
    if (server.exitCode != null) throw new Error('owned node exited before RPC');
    try { await rpc('get_tip_header'); connected = true; break; } catch { await delay(100); }
  }
  assert(connected, 'node did not become ready');
  const genesis = await rpc<{ header: { hash: ccc.Hex }; transactions: { hash: ccc.Hex; outputs: { lock: ccc.ScriptLike }[] }[] }>('get_block_by_number', ['0x0']);
  const secpDep: ccc.CellDepLike = { outPoint: { txHash: genesis.transactions[1].hash, index: 0 }, depType: 'depGroup' };
  const daoDep: ccc.CellDepLike = { outPoint: { txHash: genesis.transactions[0].hash, index: 2 }, depType: 'code' };
  // CCC's standard signer remains unmodified. Only its devnet secp dep is
  // changed from the public-network default, with all RPC fallbacks disabled.
  clientOwner = ccc.ClientPublicTestnet.open({ urls: [url] });
  const client = clientOwner.value;
  const known = client.getKnownScript.bind(client);
  client.getKnownScript = async name => name === ccc.KnownScript.Secp256k1Blake160
    ? ccc.ScriptInfo.from({ codeHash: SIGHASH, hashType: 'type', cellDeps: [{ cellDep: secpDep }] }) : known(name);
  const signer = new ccc.SignerCkbPrivateKey(client, ccc.hexFrom(new Uint8Array(32).fill(0x46)));
  const lock = (await signer.getAddressObjSecp256k1()).script;
  const deployments = JSON.parse(await readFile(join(nativeRun, 'deployments.json'), 'utf8')) as { data_hash: ccc.Hex; cell_dep: { out_point: { tx_hash: ccc.Hex; index: ccc.Hex } } }[];
  const codes = {} as Deployment['codes'];
  for (const name of Object.keys(pins) as (keyof typeof pins)[]) {
    const found = deployments.find(value => value.data_hash === pins[name]); assert(found, 'missing code ' + name);
    codes[name] = { txHash: found.cell_dep.out_point.tx_hash, index: found.cell_dep.out_point.index };
  }
  const deployment: Deployment = { genesisHash: genesis.header.hash, codes, walletDeps: [secpDep] };
  await verifyDeployment(client, deployment);
  await writeFile(join(directory, 'deployment.json'), JSON.stringify(deployment, null, 2));
  let proposalHash: ccc.Hex;
  if (resume) {
    const create = ccc.Transaction.fromBytes(await readFile(join(directory, 'proof/creation.bin')));
    proposalHash = create.hash();
    const status = await client.getTransactionNoCache(proposalHash);
    assert(status?.status === 'committed' && status.blockNumber != null && status.blockHash != null);
    assert.equal((await client.getHeaderByNumberNoCache(status.blockNumber))?.hash, status.blockHash);
    assert(await client.getCellLiveNoCache({ txHash: proposalHash, index: 0 }, true, true), 'recover live proposal');
    rows.push({ case: 'recover exact canonical CCC proposal from completed proof attempt', tx: proposalHash,
      fundedByDaoTx: create.inputs[0].previousOutput.txHash, proofAttemptPreserved: true });
  } else {
  // Fresh cellbase belongs only to this disposable chain's always-success
  // faucet. It never uses the native harness's wallet or deployed code Cells.
  const blockHash = await rpc<ccc.Hex>('generate_block');
  const block = await client.getBlockByHashNoCache(blockHash); assert(block);
  const coinbase = block.transactions[0], output = coinbase.outputs.findIndex(value => value.capacity > 200_000_000_000n);
  assert(output >= 0, 'faucet capacity');
  const faucetPoint = ccc.OutPoint.from({ txHash: coinbase.hash(), index: output });
  const faucet = ccc.Transaction.from({ inputs: [{ previousOutput: faucetPoint }], outputs: [{ capacity: coinbase.outputs[output].capacity - 100_000n, lock }], outputsData: ['0x'],
    cellDeps: [{ outPoint: { txHash: genesis.transactions[0].hash, index: 5 }, depType: 'code' }] });
  await client.sendTransactionDry(faucet, 'passthrough');
  const fundHash = await client.sendTransaction(faucet, 'passthrough');
  await mining(client.waitTransaction(fundHash, 1, 60_000));
  const fund = await client.getCellLiveNoCache({ txHash: fundHash, index: 0 }, true, true); assert(fund);
  const dao = ccc.Transaction.from({ inputs: [{ previousOutput: fund.outPoint }],
    outputs: [{ capacity: 100_000_000_000n, lock, type: { codeHash: DAO, hashType: 'type', args: '0x' } }, { capacity: fund.cellOutput.capacity - 100_000_000_000n - 100_000n, lock }],
    outputsData: [new Uint8Array(8), '0x'], cellDeps: [daoDep, secpDep] });
  const signedDao = await signer.signOnlyTransaction(await signer.prepareTransaction(dao));
  await client.sendTransactionDry(signedDao, 'passthrough');
  const daoHash = await client.sendTransaction(signedDao, 'passthrough');
  await mining(client.waitTransaction(daoHash, 1, 60_000));
  rows.push({ case: 'CCC signed standard DAO deposit', tx: daoHash });
  const creation = await prepareCreation(signer, { txHash: daoHash, index: 1 }, deployment,
    { duration: 16, quorum: 50_000_000_000n, amount: 10_000_000_000n, recipient: lock, description: ccc.hashCkb(new TextEncoder().encode('CCC public fixture')) });
  proposalHash = await mining(creation.signAndSend(signer, 1, 60_000));
  rows.push({ case: 'CCC signed funded proposal', tx: proposalHash });
  await assert.rejects(creation.checkFresh(), error => error instanceof AgoraError && error.code === 'stale_input');
  const vote = await prepareVote(signer, { txHash: proposalHash, index: 1 }, { txHash: daoHash, index: 0 }, { txHash: proposalHash, index: 0 }, true, deployment);
  const voteHash = await mining(vote.signAndSend(signer, 1, 60_000));
  rows.push({ case: 'CCC signed eligible YES ballot', tx: voteHash });
  await writeFile(join(directory, 'journal.json'), JSON.stringify(rows, null, 2));
  }
  const p = await client.getTransactionNoCache(proposalHash); assert(p?.blockNumber != null);
  const endNumber = p.blockNumber + 16n;
  const proofDirectory = join(directory, 'proof');
  if (!resume) {
    while ((await client.getTipHeader()).number < endNumber) await rpc('generate_block');
    await tool('target/debug/agoraseal-export', [url, proposalHash, '0', proofDirectory]);
  }
  await tool('target/debug/agoraseal-prove-local', [proofDirectory]);
  const proof = await readFile(join(proofDirectory, 'proof-plonk.bin')), publicBytes = await readFile(join(proofDirectory, 'public-values.bin'));
  const statement = decodeStatement(publicBytes); assert(statement.passed); assert.equal(statement.yes, 100_000_000_000n);
  const originalCell = await client.getCellLiveNoCache({ txHash: proposalHash, index: 0 }, true, true); assert(originalCell);
  const concat = (...values: Uint8Array[]) => Buffer.concat(values);
  const receipt = originalCell.cellOutput.clone(); receipt.capacity = 603n * 100_000_000n;
  const staleTransaction = ccc.Transaction.from({ inputs: [{ previousOutput: originalCell.outPoint }],
    outputs: [receipt, { capacity: decodeProposal(originalCell.outputData).amount, lock }], outputsData: [concat(ccc.bytesFrom(originalCell.outputData), publicBytes), '0x'],
    cellDeps: ['sp1', 'stateContext', 'proposal', 'treasury'].map(name => ({ outPoint: deployment.codes[name as keyof typeof pins], depType: 'code' })),
    headerDeps: [statement.startHash, statement.endHash, statement.genesis] });
  staleTransaction.setWitnessArgs(0, { inputType: concat(new TextEncoder().encode('CSARGv1\0'), ccc.bytesFrom(ccc.hashCkb(publicBytes)), ccc.bytesFrom(lock.args), ccc.bytesFrom(lock.hash())),
    outputType: concat(proof, publicBytes) });
  await writeFile(join(directory, 'stale-settlement.bin'), staleTransaction.toBytes());
  // Rewind only this attempt's last end block. Node and SDK must reject a
  // now noncanonical proof. Permanent replacement requires a new proof.
  const predecessor = await client.getHeaderByNumberNoCache(endNumber - 1n); assert(predecessor);
  await rpc('truncate', [predecessor.hash]);
  await assert.rejects(prepareSettlement(client, originalCell.outPoint, lock, proof, publicBytes, lock, deployment), error => error instanceof AgoraError && error.code === 'reorg');
  await assert.rejects(client.sendTransaction(staleTransaction, 'passthrough'));
  const template = await rpc<Record<string, unknown>>('get_block_template');
  template.current_time = '0x' + (BigInt(template.current_time as string) + 1n).toString(16);
  const replacement = await rpc<ccc.Hex>('generate_block_with_template', [template]); assert.notEqual(replacement, statement.endHash);
  await assert.rejects(prepareSettlement(client, originalCell.outPoint, lock, proof, publicBytes, lock, deployment), error => error instanceof AgoraError && error.code === 'reorg');
  const substituted = staleTransaction.clone(); substituted.headerDeps[1] = replacement;
  await assert.rejects(client.sendTransactionDry(substituted, 'passthrough'), error => error instanceof ccc.ErrorClientVerification && error.errorCode === 1);
  rows.push({ case: 'canonical-end removal/replacement rejected by SDK, normal node admission and context; preserve old proof and regenerate', old: statement.endHash, replacement });
  await writeFile(join(directory, 'journal-reorg.json'), JSON.stringify(rows, null, 2));
  const recoveredProof = join(directory, 'proof-after-reorg');
  await tool('target/debug/agoraseal-export', [url, proposalHash, '0', recoveredProof]);
  await tool('target/debug/agoraseal-prove-local', [recoveredProof]);
  const freshPublic = await readFile(join(recoveredProof, 'public-values.bin'));
  assert.equal(decodeStatement(freshPublic).endHash, replacement);
  const settlement = await prepareSettlement(client, originalCell.outPoint, lock, await readFile(join(recoveredProof, 'proof-plonk.bin')), freshPublic, lock, deployment);
  const settleHash = await mining(settlement.sendSettlement(1, 60_000));
  rows.push({ case: 'CCC permissionless PLONK settlement', tx: settleHash });
  await assert.rejects(settlement.sendSettlement(1, 5000), error => error instanceof AgoraError && error.code === 'stale_input');
  const paid = await client.getCellLiveNoCache({ txHash: settleHash, index: 1 }, true, true); assert(paid);
  const spend = ccc.Transaction.from({ inputs: [{ previousOutput: paid.outPoint }], outputs: [{ capacity: paid.cellOutput.capacity - 100_000n, lock }], outputsData: ['0x'], cellDeps: [secpDep] });
  const signedSpend = await signer.signOnlyTransaction(await signer.prepareTransaction(spend));
  await client.sendTransactionDry(signedSpend, 'passthrough');
  const spendHash = await client.sendTransaction(signedSpend, 'passthrough'); await mining(client.waitTransaction(spendHash, 1, 60_000));
  rows.push({ case: 'CCC signed payment spend', tx: spendHash });
  await writeFile(join(directory, 'settlement.bin'), settlement.transaction.toBytes());
  await writeFile(join(directory, 'evidence.json'), JSON.stringify({ schema: 'agoraseal-ccc-node-rehearsal-v1', status: 'passed', production_admitted: false,
    wallet: 'unmodified CCC SignerCkbPrivateKey; public scalar 0x46 repeated; disposable chain only', deployment, rows }, null, 2));
  console.log('CCC node lifecycle and reorg rehearsal passed:', directory);
} finally {
  try { await clientOwner?.dispose(); }
  finally { server.stdin.end(); await serverClosed; }
}

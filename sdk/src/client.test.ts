import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { ccc } from '@ckb-ccc/shell';
import { AgoraError, DAO, PreparedTransaction, SIGHASH, decodeProposal, decodeStatement, encodeProposal,
  pins, prepareCreation, prepareSettlement, prepareVote, verifyDeployment, type Deployment } from './client.ts';

const h = (byte: number) => ccc.hexFrom(new Uint8Array(32).fill(byte));
const recipient = ccc.Script.from({ codeHash: SIGHASH, hashType: 'type', args: ccc.hexFrom(new Uint8Array(20).fill(9)) });
const fixture = (name: string) => readFileSync(new URL('../../tests/fixtures/funded-failed/' + name, import.meta.url));
const creation = ccc.Transaction.fromBytes(fixture('creation.bin'));
const proposal = ccc.Cell.from({ outPoint: { txHash: creation.hash(), index: 0 }, cellOutput: creation.outputs[0], outputData: creation.outputsData[0] });
const statement = decodeStatement(fixture('public-values.bin'));
const codePaths = {
  sp1: '../../verifiers/sp1-plonk/target/riscv64imac-unknown-none-elf/release/agoraseal-sp1-plonk',
  stateContext: '../../verifiers/ckb-state-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-state-context',
  voteContext: '../../verifiers/ckb-context/target/riscv64imac-unknown-none-elf/release/agoraseal-ckb-context',
  vote: '../../target/cellscript/vote.elf', proposal: '../../target/cellscript/funded-proposal.elf', treasury: '../../target/cellscript/funded-treasury.elf',
} as const;
function environment() {
  const cells = new Map<string, ccc.Cell>();
  const codes = {} as Deployment['codes'];
  Object.entries(codePaths).forEach(([name, path], index) => {
    const point = ccc.OutPoint.from({ txHash: h(index + 30), index: 0 });
    codes[name as keyof typeof pins] = point;
    const cell = ccc.Cell.from({ outPoint: point, cellOutput: { capacity: 50_000_000_000_000n, lock: recipient }, outputData: readFileSync(new URL(path, import.meta.url)) });
    cells.set(point.toHex(), cell);
  });
  cells.set(proposal.outPoint.toHex(), proposal.clone());
  const funding = ccc.Cell.from({ outPoint: { txHash: h(80), index: 0 }, cellOutput: { capacity: 1_000_000_000_000n, lock: recipient }, outputData: '0x' });
  cells.set(funding.outPoint.toHex(), funding);
  const deposit = ccc.Cell.from({ outPoint: { txHash: h(81), index: 0 }, cellOutput: { capacity: 100_000_000_000n, lock: recipient, type: { codeHash: DAO, hashType: 'type', args: '0x' } }, outputData: new Uint8Array(8) });
  cells.set(deposit.outPoint.toHex(), deposit);
  const headers = new Map<bigint, ccc.Hex>([[0n, statement.genesis], [99n, h(99)], [100n, statement.startHash], [102n, statement.endHash]]);
  let tip = 100n, sent = 0;
  const client = {
    getHeaderByNumberNoCache: async (n: ccc.NumLike) => headers.has(ccc.numFrom(n)) ? { hash: headers.get(ccc.numFrom(n)), number: ccc.numFrom(n) } : undefined,
    getCellLiveNoCache: async (point: ccc.OutPointLike) => cells.get(ccc.OutPoint.from(point).toHex())?.clone(),
    getTransactionNoCache: async (txHash: ccc.HexLike) => ({ status: 'committed', blockNumber: ccc.hexFrom(txHash) === deposit.outPoint.txHash ? 99n : 100n,
      blockHash: ccc.hexFrom(txHash) === deposit.outPoint.txHash ? h(99) : statement.startHash }),
    getTipHeader: async () => ({ number: tip }),
    sendTransactionDry: async () => 66_600_000n,
    sendTransaction: async (tx: ccc.Transaction) => { sent++; return tx.hash(); },
    waitTransaction: async () => ({ status: 'committed', blockNumber: 102n, blockHash: statement.endHash }),
  } as unknown as ccc.Client;
  const signer = { client, prepareTransaction: async (tx: ccc.Transaction) => {
    const witness = tx.getWitnessArgs(0)!; witness.lock = ccc.hexFrom(new Uint8Array(65)); tx.setWitnessArgs(0, witness); return tx;
  } } as unknown as ccc.Signer;
  const deployment: Deployment = { genesisHash: statement.genesis, codes };
  return { client, signer, cells, funding, deposit, headers, deployment, setTip: (n: bigint) => { tip = n; }, sent: () => sent };
}
const rejects = (code: string) => (error: unknown) => error instanceof AgoraError && error.code === code;
const parameters = { duration: 2, quorum: 1n, amount: 10_000_000_000n, recipient, description: h(12) };

test('CCC codecs agree with independently generated Rust/VM golden bytes and Type-ID', () => {
  const e = environment();
  assert.deepEqual(Buffer.from(encodeProposal(e.deployment, parameters)), Buffer.from(ccc.bytesFrom(proposal.outputData)));
  assert.equal(decodeProposal(proposal.outputData).daoHash, ccc.Script.from({ codeHash: DAO, hashType: 'type', args: '0x' }).hash());
  assert(statement.proposalPoint.eq(proposal.outPoint));
  assert.equal(statement.proposalScript, proposal.cellOutput.type!.hash());
  assert.equal(statement.proposalDataHash, ccc.hashCkb(proposal.outputData));
  assert.equal(statement.passed, false);
  assert.equal(statement.endNumber - statement.startNumber, 2n);
});
test('codecs reject trailing bytes, invalid boolean and overflowing integers', () => {
  assert.throws(() => decodeProposal(Buffer.concat([Buffer.from(ccc.bytesFrom(proposal.outputData)), Buffer.of(0)])), rejects('proposal'));
  const publicBytes = fixture('public-values.bin'); publicBytes[276] = 2;
  assert.throws(() => decodeStatement(publicBytes), rejects('statement'));
  assert.throws(() => encodeProposal(environment().deployment, { ...parameters, amount: 1n << 64n }), rejects('amount'));
  assert.throws(() => encodeProposal(environment().deployment, { ...parameters, duration: 17 }), rejects('duration'));
});
test('deployment must match genesis and actual code bytes, independently of supplied descriptors', async () => {
  const e = environment(); await verifyDeployment(e.client, e.deployment);
  await assert.rejects(verifyDeployment(e.client, { ...e.deployment, genesisHash: h(3) }), rejects('wrong_chain'));
  const point = ccc.OutPoint.from(e.deployment.codes.sp1); e.cells.get(point.toHex())!.outputData = '0x01';
  await assert.rejects(verifyDeployment(e.client, e.deployment), rejects('code_identity'));
});
test('creation fixes 603 CKB receipt reserve, refund owner, Type-ID and witness independently of wallet', async () => {
  const e = environment(); const ready = await prepareCreation(e.signer, e.funding.outPoint, e.deployment, parameters);
  const tx = ready.transaction;
  assert.equal(tx.outputs[0].capacity, 603n * 100_000_000n + parameters.amount + 100_000n);
  assert.equal(tx.outputs[0].lock.args, recipient.hash());
  const idInput = new Uint8Array(52); idInput.set(tx.inputs[0].toBytes());
  assert.equal(tx.outputs[0].type!.args, ccc.hashCkb(idInput));
  assert.equal(ccc.bytesFrom(tx.getWitnessArgs(0)!.inputType!).length, 92);
  tx.outputs[0].capacity++; assert.notEqual(ready.transaction.hash(), tx.hash());
});
test('wallet preparation may fill signature placeholder but cannot alter policy or since', async () => {
  for (const change of ['since', 'entry', 'slots']) {
    const e = environment();
    e.signer.prepareTransaction = async txLike => {
      const tx = ccc.Transaction.from(txLike);
      if (change === 'since') tx.inputs[0].since = 1n;
      if (change === 'entry') tx.setWitnessArgs(0, { inputType: '0x' });
      if (change === 'slots') tx.cellDeps.reverse();
      return tx;
    };
    await assert.rejects(prepareCreation(e.signer, e.funding.outPoint, e.deployment, parameters), rejects(change === 'slots' ? 'dependency_order' : 'wallet_mutation'));
  }
});
test('signed transaction cannot change treasury state, dependency order, proof or policy witnesses', async () => {
  const e = environment(); const ready = await prepareCreation(e.signer, e.funding.outPoint, e.deployment, parameters);
  const signed = ready.transaction; signed.setWitnessArgs(0, { ...signed.getWitnessArgs(0)!, lock: ccc.hexFrom(new Uint8Array(65).fill(3)) }); ready.checkSigned(signed);
  signed.inputs[0].since = 2n; assert.throws(() => ready.checkSigned(signed), rejects('wallet_mutation'));
  const changed = ready.transaction; changed.setWitnessArgs(0, { ...changed.getWitnessArgs(0)!, inputType: '0x' });
  assert.throws(() => ready.checkSigned(changed), rejects('wallet_mutation'));
});
test('freshness bypasses caches and rejects a spent funding Cell or changed code', async () => {
  const e = environment(); const ready = await prepareCreation(e.signer, e.funding.outPoint, e.deployment, parameters);
  e.cells.delete(e.funding.outPoint.toHex()); await assert.rejects(ready.checkFresh(), rejects('stale_input'));
  assert.equal(e.sent(), 0);
});
test('vote uses real DAO capacity and creation anchors, rejects assets, expired windows and reorgs', async () => {
  const e = environment(); const ready = await prepareVote(e.signer, e.funding.outPoint, e.deposit.outPoint, proposal.outPoint, true, e.deployment);
  const tx = ready.transaction;
  assert(tx.cellDeps[0].outPoint.eq(e.deposit.outPoint)); assert(tx.cellDeps[1].outPoint.eq(proposal.outPoint));
  assert.equal(ccc.bytesFrom(tx.outputsData[0]).length, 53);
  e.setTip(102n); await assert.rejects(ready.checkFresh(), rejects('window_closed'));
  e.setTip(100n); e.headers.set(100n, h(6)); await assert.rejects(ready.checkFresh(), rejects('reorg'));
  e.headers.set(100n, statement.startHash); e.funding.outputData = '0x01';
  await assert.rejects(prepareVote(e.signer, e.funding.outPoint, e.deposit.outPoint, proposal.outPoint, true, e.deployment), rejects('funding'));
});
test('failed settlement matches Rust golden input and raw witness layout; anchors must remain canonical', async () => {
  const e = environment();
  // Golden refund Lock is intentionally always-success: this unit test does not claim wallet/node admission.
  const refund = ccc.Script.fromBytes(fixture('refund-script.bin'));
  const ready = await prepareSettlement(e.client, proposal.outPoint, recipient, fixture('proof-plonk.bin'), fixture('public-values.bin'), refund, e.deployment);
  const tx = ready.transaction;
  assert.equal(tx.inputs.length, 1); assert.equal(tx.outputs.length, 2);
  assert.equal(tx.outputs[0].capacity, 603n * 100_000_000n);
  assert.equal(tx.outputs[1].capacity, proposal.cellOutput.capacity - tx.outputs[0].capacity - 10_000n);
  assert(tx.outputs[1].lock.eq(refund));
  assert.equal(ccc.bytesFrom(tx.getWitnessArgs(0)!.outputType!).length, 1241);
  assert.equal(ccc.bytesFrom(tx.getWitnessArgs(0)!.inputType!).length, 92);
  e.headers.set(102n, h(7)); await assert.rejects(ready.checkFresh(), rejects('reorg'));
  await assert.rejects(ready.sendSettlement(), rejects('reorg')); assert.equal(e.sent(), 0);
});
test('client sends settlement only after dry-run and fresh canonical confirmation', async () => {
  const e = environment(); const ready = await prepareSettlement(e.client, proposal.outPoint, recipient, fixture('proof-plonk.bin'), fixture('public-values.bin'), ccc.Script.fromBytes(fixture('refund-script.bin')), e.deployment);
  assert.equal(await ready.sendSettlement(), ready.transaction.hash()); assert.equal(e.sent(), 1);
});
test('dry-run failure or excessive cycles prevents broadcast; host does not accept proof as authority', async () => {
  for (const mode of ['reject', 'cycles']) {
    const e = environment(); const ready = await prepareSettlement(e.client, proposal.outPoint, recipient, fixture('proof-plonk.bin'), fixture('public-values.bin'), ccc.Script.fromBytes(fixture('refund-script.bin')), e.deployment);
    e.client.sendTransactionDry = async () => { if (mode === 'reject') throw new Error('VM rejected proof'); return 70_000_001n; };
    await assert.rejects(ready.sendSettlement(), rejects(mode === 'reject' ? 'dry_run' : 'cycle_budget')); assert.equal(e.sent(), 0);
  }
});

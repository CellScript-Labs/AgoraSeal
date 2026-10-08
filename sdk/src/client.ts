import { ccc } from '@ckb-ccc/shell';
import { pins } from './pins.ts';
export { pins, programKey } from './pins.ts';

const SH = 100_000_000n, MAX = 0xffffffffffffffffn;
export const SIGHASH = '0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8' as const;
export const DAO = '0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e' as const;
export interface Deployment {
  genesisHash: ccc.HexLike;
  codes: Record<keyof typeof pins, ccc.OutPointLike>;
  walletDeps?: ccc.CellDepLike[];
}
export class AgoraError extends Error {
  readonly code: string;
  constructor(code: string, message: string) { super(message); this.name = 'AgoraError'; this.code = code; }
}
function need(ok: unknown, code: string, message: string): asserts ok {
  if (!ok) throw new AgoraError(code, message);
}
const bytes = ccc.bytesFrom, hex = ccc.hexFrom;
const same = (a: ccc.HexLike, b: ccc.HexLike) => hex(a) === hex(b);
const cat = (...parts: Uint8Array[]) => {
  const out = new Uint8Array(parts.reduce((n, value) => n + value.length, 0));
  let offset = 0; for (const part of parts) { out.set(part, offset); offset += part.length; } return out;
};
const le = (value: bigint, width = 8) => {
  need(value >= 0n && value <= (1n << BigInt(width * 8)) - 1n, 'encoding', 'integer exceeds codec width');
  const out = new Uint8Array(width); for (let i = 0; i < width; i++) out[i] = Number((value >> BigInt(i * 8)) & 255n); return out;
};
const u64 = (value: Uint8Array, offset: number) => new DataView(value.buffer, value.byteOffset, value.byteLength).getBigUint64(offset, true);
const hash32 = (value: ccc.HexLike) => { const result = bytes(value); need(result.length === 32, 'encoding', 'expected 32 bytes'); return result; };
const magic = (text: string) => new TextEncoder().encode(text);
const occupied = (output: ccc.CellOutput, dataBytes: number) => BigInt(output.occupiedSize + dataBytes) * SH;
function recipientScript(value: ccc.ScriptLike): ccc.Script {
  const script = ccc.Script.from(value);
  need(script.codeHash === SIGHASH && script.hashType === 'type' && bytes(script.args).length === 20,
    'recipient', 'recipient must be standard secp256k1 with 20-byte arguments');
  return script;
}
function entry(claim: ccc.HexLike, recipient: ccc.Script): Uint8Array {
  return cat(magic('CSARGv1\0'), hash32(claim), bytes(recipient.args), hash32(recipient.hash()));
}
function dep(deployment: Deployment, name: keyof typeof pins): ccc.CellDepLike {
  return { outPoint: deployment.codes[name], depType: 'code' };
}
async function live(client: ccc.Client, outPoint: ccc.OutPointLike): Promise<ccc.Cell> {
  const cell = await client.getCellLiveNoCache(outPoint, true, true);
  need(cell && cell.outPoint.eq(outPoint), 'stale_input', 'Cell is spent, missing or changed; refresh before retrying');
  return cell;
}
async function confirmed(client: ccc.Client, point: ccc.OutPoint): Promise<ccc.ClientTransactionResponse> {
  const tx = await client.getTransactionNoCache(point.txHash);
  need(tx?.status === 'committed' && tx.blockNumber != null && tx.blockHash != null,
    'unconfirmed', 'Cell creation transaction must be committed');
  const header = await client.getHeaderByNumberNoCache(tx.blockNumber);
  need(header && header.hash === tx.blockHash, 'reorg', 'creation header is no longer canonical');
  return tx;
}
export async function verifyDeployment(client: ccc.Client, deployment: Deployment): Promise<void> {
  const genesis = await client.getHeaderByNumberNoCache(0);
  need(genesis && same(genesis.hash, deployment.genesisHash), 'wrong_chain', 'deployment genesis differs from this node');
  for (const name of Object.keys(pins) as (keyof typeof pins)[]) {
    const cell = await live(client, deployment.codes[name]);
    need(ccc.hashCkb(cell.outputData) === pins[name], 'code_identity', name + ' code differs from the fixed artifact');
    await confirmed(client, cell.outPoint);
  }
}
export interface ProposalParameters {
  duration: number; quorum: bigint; amount: bigint; recipient: ccc.ScriptLike; description: ccc.HexLike;
}
export interface ProposalData {
  genesis: ccc.Hex; voteCode: ccc.Hex; duration: number; quorum: bigint; amount: bigint;
  recipientHash: ccc.Hex; description: ccc.Hex; daoHash: ccc.Hex;
}
export function encodeProposal(deployment: Deployment, params: ProposalParameters): Uint8Array {
  need(Number.isSafeInteger(params.duration) && params.duration >= 1 && params.duration <= 16, 'duration', 'duration must be 1..16 blocks');
  need(params.quorum > 0n && params.quorum <= MAX && params.amount >= 61n * SH && params.amount <= MAX, 'amount', 'invalid quorum or amount');
  const recipient = recipientScript(params.recipient);
  const dao = ccc.Script.from({ codeHash: DAO, hashType: 'type', args: '0x' });
  return cat(magic('AGPROP01'), hash32(deployment.genesisHash), hash32(pins.vote), le(BigInt(params.duration), 4),
    le(params.quorum), le(params.amount), hash32(recipient.hash()), hash32(params.description), hash32(dao.hash()));
}
export function decodeProposal(value: ccc.HexLike): ProposalData {
  const data = bytes(value);
  need(data.length === 188 && same(data.slice(0, 8), magic('AGPROP01')), 'proposal', 'expected 188-byte live proposal');
  const result = {
    genesis: hex(data.slice(8, 40)), voteCode: hex(data.slice(40, 72)),
    duration: new DataView(data.buffer, data.byteOffset, data.byteLength).getUint32(72, true),
    quorum: u64(data, 76), amount: u64(data, 84), recipientHash: hex(data.slice(92, 124)),
    description: hex(data.slice(124, 156)), daoHash: hex(data.slice(156, 188)),
  };
  need(result.duration >= 1 && result.duration <= 16 && result.quorum > 0n && result.amount >= 61n * SH, 'proposal', 'inadmissible proposal parameters');
  return result;
}
export interface PublicStatement {
  genesis: ccc.Hex; proposalScript: ccc.Hex; proposalPoint: ccc.OutPoint; proposalDataHash: ccc.Hex;
  startHash: ccc.Hex; endHash: ccc.Hex; startNumber: bigint; endNumber: bigint;
  yes: bigint; no: bigint; counted: bigint; countedDigest: ccc.Hex; passed: boolean;
}
export function decodeStatement(value: ccc.HexLike): PublicStatement {
  const data = bytes(value);
  need(data.length === 277 && same(data.slice(0, 8), magic('AGZKPV01')) && data[276] <= 1, 'statement', 'invalid 277-byte statement');
  return {
    genesis: hex(data.slice(8, 40)), proposalScript: hex(data.slice(40, 72)),
    proposalPoint: ccc.OutPoint.from({ txHash: hex(data.slice(72, 104)), index: new DataView(data.buffer, data.byteOffset, data.byteLength).getUint32(104, true) }),
    proposalDataHash: hex(data.slice(108, 140)), startHash: hex(data.slice(140, 172)), endHash: hex(data.slice(172, 204)),
    startNumber: u64(data, 204), endNumber: u64(data, 212), yes: u64(data, 220), no: u64(data, 228),
    counted: u64(data, 236), countedDigest: hex(data.slice(244, 276)), passed: data[276] === 1,
  };
}
async function proposalCell(client: ccc.Client, point: ccc.OutPointLike, deployment: Deployment): Promise<ccc.Cell> {
  const cell = await live(client, point);
  need(cell.cellOutput.type?.codeHash === pins.proposal && cell.cellOutput.type.hashType === 'data2'
    && bytes(cell.cellOutput.type.args).length === 32, 'proposal_identity', 'proposal Type differs from the fixed lifecycle');
  need(cell.cellOutput.lock.codeHash === pins.treasury && cell.cellOutput.lock.hashType === 'data2'
    && bytes(cell.cellOutput.lock.args).length === 32, 'treasury_identity', 'proposal funding Lock differs from the fixed lifecycle');
  const params = decodeProposal(cell.outputData);
  need(same(params.genesis, deployment.genesisHash) && params.voteCode === pins.vote
    && params.daoHash === ccc.Script.from({ codeHash: DAO, hashType: 'type', args: '0x' }).hash(),
    'proposal_identity', 'proposal chain/vote/DAO identities differ');
  return cell;
}
function checkSlots(tx: ccc.Transaction, deployment: Deployment, names: (keyof typeof pins)[]): void {
  names.forEach((name, index) => need(tx.cellDeps[index]?.depType === 'code' && tx.cellDeps[index].outPoint.eq(deployment.codes[name]),
    'dependency_order', 'exact ' + name + ' dependency slot ' + index + ' changed'));
}
function checkPreparedWitnesses(original: ccc.Transaction, prepared: ccc.Transaction): void {
  need(prepared.witnesses.length === original.witnesses.length, 'wallet_mutation', 'wallet changed policy witness count');
  original.witnesses.forEach((_, index) => {
    const before = original.getWitnessArgs(index), after = prepared.getWitnessArgs(index);
    need(before && after && before.inputType === after.inputType && before.outputType === after.outputType,
      'wallet_mutation', 'wallet changed a policy witness while preparing');
  });
}
export class PreparedTransaction {
  #tx: ccc.Transaction; #cells: ccc.Cell[]; #deployment: Deployment;
  #client: ccc.Client; #refresh: () => Promise<void>; #kind: 'create' | 'vote' | 'settle';
  constructor(kind: 'create' | 'vote' | 'settle', client: ccc.Client, tx: ccc.Transaction, cells: ccc.Cell[], deployment: Deployment, refresh: () => Promise<void>) {
    this.#kind = kind; this.#client = client; this.#tx = tx.clone(); this.#cells = cells.map(cell => cell.clone());
    this.#deployment = structuredClone(deployment); this.#refresh = refresh;
  }
  get transaction(): ccc.Transaction { return this.#tx.clone(); }
  checkSigned(signed: ccc.Transaction): void {
    need(signed.hash() === this.#tx.hash(), 'wallet_mutation', 'wallet changed the finalized transaction');
    need(signed.witnesses.length === this.#tx.witnesses.length, 'wallet_mutation', 'wallet changed witness count');
    this.#tx.witnesses.forEach((before, index) => {
      if (before === signed.witnesses[index]) return;
      const a = this.#tx.getWitnessArgs(index), b = signed.getWitnessArgs(index);
      need(a && b && a.inputType === b.inputType && a.outputType === b.outputType, 'wallet_mutation', 'wallet changed a policy/proof witness');
    });
  }
  async checkFresh(): Promise<void> {
    await verifyDeployment(this.#client, this.#deployment);
    for (const old of this.#cells) {
      const fresh = await live(this.#client, old.outPoint);
      need(same(fresh.cellOutput.toBytes(), old.cellOutput.toBytes()) && fresh.outputData === old.outputData, 'stale_input', 'resolved Cell changed');
    }
    await this.#refresh();
  }
  async #broadcast(tx: ccc.Transaction, confirmations: number, timeout: number): Promise<ccc.Hex> {
    this.checkSigned(tx); await this.checkFresh();
    let cycles: bigint;
    try { cycles = await this.#client.sendTransactionDry(tx, 'passthrough'); }
    catch (error) { throw new AgoraError('dry_run', 'CKB policy/proof execution rejected: ' + String(error)); }
    need(cycles <= 70_000_000n, 'cycle_budget', 'transaction exceeds the 70M client admission budget');
    const hash = await this.#client.sendTransaction(tx, 'passthrough');
    const result = await this.#client.waitTransaction(hash, confirmations, timeout);
    need(result?.status === 'committed' && result.blockNumber != null && result.blockHash != null, 'confirmation', 'transaction did not commit');
    const canonical = await this.#client.getHeaderByNumberNoCache(result.blockNumber);
    need(canonical?.hash === result.blockHash, 'reorg', 'transaction confirmation became noncanonical');
    return hash;
  }
  async signAndSend(signer: ccc.Signer, confirmations = 1, timeout = 120_000): Promise<ccc.Hex> {
    need(signer.client === this.#client, 'wrong_client', 'signer uses another client');
    await this.checkFresh();
    return this.#broadcast(await signer.signOnlyTransaction(this.#tx.clone()), confirmations, timeout);
  }
  async sendSettlement(confirmations = 1, timeout = 120_000): Promise<ccc.Hex> {
    need(this.#kind === 'settle', 'signature', 'funding/voting requires wallet signing');
    return this.#broadcast(this.#tx.clone(), confirmations, timeout);
  }
}
export async function prepareCreation(signer: ccc.Signer, fundingPoint: ccc.OutPointLike, deployment: Deployment, params: ProposalParameters, fee = 100_000n): Promise<PreparedTransaction> {
  await verifyDeployment(signer.client, deployment);
  const funding = await live(signer.client, fundingPoint);
  need(!funding.cellOutput.type && bytes(funding.outputData).length === 0, 'funding', 'funding Cell must be ordinary and empty');
  need(params.amount >= occupied(funding.cellOutput, 0), 'amount', 'amount cannot cover refund Lock capacity');
  need(fee >= 0n && fee <= MAX, 'fee', 'invalid creation fee');
  const recipient = recipientScript(params.recipient), data = encodeProposal(deployment, params);
  const input = ccc.CellInput.from({ previousOutput: funding.outPoint, since: 0n });
  const id = ccc.hashCkb(cat(input.toBytes(), le(0n)));
  const type = ccc.Script.from({ codeHash: pins.proposal, hashType: 'data2', args: id });
  const lock = ccc.Script.from({ codeHash: pins.treasury, hashType: 'data2', args: funding.cellOutput.lock.hash() });
  const output = ccc.CellOutput.from({ capacity: 0n, type, lock });
  output.capacity = occupied(output, 465) + params.amount + 100_000n;
  const change = funding.cellOutput.capacity - output.capacity - fee;
  need(change >= occupied(funding.cellOutput, 0), 'capacity', 'funding Cell cannot cover reserve, receipt, fee and change');
  let tx = ccc.Transaction.from({ inputs: [input], outputs: [output, { capacity: change, lock: funding.cellOutput.lock }], outputsData: [data, '0x'],
    cellDeps: ['sp1', 'stateContext', 'proposal', 'treasury'].map(name => dep(deployment, name as keyof typeof pins)),
    headerDeps: [deployment.genesisHash] });
  for (const value of deployment.walletDeps ?? []) tx.addCellDeps(value);
  tx.setWitnessArgs(0, { inputType: entry(new Uint8Array(32), recipient) });
  const original = tx.clone(); tx = await signer.prepareTransaction(tx);
  need(tx.version === original.version && tx.inputs.length === 1 && same(tx.inputs[0].toBytes(), original.inputs[0].toBytes())
    && tx.outputs.length === 2 && tx.outputs.every((value, i) => same(value.toBytes(), original.outputs[i].toBytes()))
    && tx.outputsData.length === 2 && tx.outputsData.every((value, i) => value === original.outputsData[i])
    && tx.headerDeps.length === 1 && same(tx.headerDeps[0], deployment.genesisHash),
    'wallet_mutation', 'wallet altered funding selection, headers or proposal outputs');
  checkSlots(tx, deployment, ['sp1', 'stateContext', 'proposal', 'treasury']);
  checkPreparedWitnesses(original, tx);
  need(fee >= tx.estimateFee(1000n), 'fee', 'creation fee below estimated 1000-shannon/KB policy');
  return new PreparedTransaction('create', signer.client, tx, [funding], deployment, async () => {});
}
export async function prepareVote(signer: ccc.Signer, fundingPoint: ccc.OutPointLike, depositPoint: ccc.OutPointLike, proposalPoint: ccc.OutPointLike, choice: boolean, deployment: Deployment, fee = 100_000n): Promise<PreparedTransaction> {
  await verifyDeployment(signer.client, deployment);
  const funding = await live(signer.client, fundingPoint), deposit = await live(signer.client, depositPoint), proposal = await proposalCell(signer.client, proposalPoint, deployment);
  need(!funding.cellOutput.type && bytes(funding.outputData).length === 0, 'funding', 'voting funding Cell must be ordinary and empty');
  const params = decodeProposal(proposal.outputData), d = await confirmed(signer.client, deposit.outPoint), p = await confirmed(signer.client, proposal.outPoint);
  need(d.blockNumber! < p.blockNumber!, 'snapshot', 'DAO deposit must predate the proposal');
  need(deposit.cellOutput.type?.hash() === params.daoHash && bytes(deposit.outputData).length === 8 && bytes(deposit.outputData).every(byte => byte === 0), 'dao', 'Cell is not a deposit-phase standard DAO Cell');
  need(funding.cellOutput.lock.eq(deposit.cellOutput.lock) && !funding.outPoint.eq(deposit.outPoint), 'owner', 'voting funding must use the deposit owner Lock without spending the deposit');
  const refresh = async () => {
    const tip = await signer.client.getTipHeader();
    need(tip.number < p.blockNumber! + BigInt(params.duration), 'window_closed', 'voting window is closed; this ballot cannot count');
    const current = await confirmed(signer.client, proposal.outPoint), currentDeposit = await confirmed(signer.client, deposit.outPoint);
    need(current.blockHash === p.blockHash && currentDeposit.blockHash === d.blockHash, 'reorg', 'proposal/deposit creation header changed');
    await live(signer.client, deposit.outPoint);
  };
  await refresh();
  const data = cat(magic('AGVOTE01'), deposit.outPoint.toBytes(), le(deposit.cellOutput.capacity), new Uint8Array([choice ? 1 : 0]));
  const output = ccc.CellOutput.from({ capacity: 0n, lock: deposit.cellOutput.lock, type: { codeHash: pins.vote, hashType: 'data2', args: proposal.cellOutput.type!.hash() } });
  output.capacity = occupied(output, 53);
  const change = funding.cellOutput.capacity - output.capacity - fee;
  need(fee >= 0n && change >= occupied(funding.cellOutput, bytes(funding.outputData).length), 'capacity', 'funding cannot cover ballot and fee');
  let tx = ccc.Transaction.from({ inputs: [{ previousOutput: funding.outPoint }], outputs: [output, { capacity: change, lock: funding.cellOutput.lock }], outputsData: [data, '0x'],
    cellDeps: [{ outPoint: deposit.outPoint, depType: 'code' }, { outPoint: proposal.outPoint, depType: 'code' }, dep(deployment, 'voteContext'), dep(deployment, 'vote')],
    headerDeps: [d.blockHash!, p.blockHash!] });
  for (const value of deployment.walletDeps ?? []) tx.addCellDeps(value);
  tx.setWitnessArgs(0, { inputType: cat(magic('CSARGv1\0'), le(d.blockNumber!), le(p.blockNumber!)) });
  const original = tx.clone(); tx = await signer.prepareTransaction(tx);
  need(tx.version === original.version && tx.inputs.length === 1 && same(tx.inputs[0].toBytes(), original.inputs[0].toBytes()) && tx.outputs.length === 2
    && tx.outputs.every((value, i) => same(value.toBytes(), original.outputs[i].toBytes()))
    && tx.outputsData.length === 2 && tx.outputsData.every((value, i) => value === original.outputsData[i])
    && tx.headerDeps.length === 2 && tx.headerDeps.every((value, i) => value === original.headerDeps[i]),
    'wallet_mutation', 'wallet changed ballot inputs, headers or outputs');
  need(tx.cellDeps[0]?.outPoint.eq(deposit.outPoint) && tx.cellDeps[0].depType === 'code'
    && tx.cellDeps[1]?.outPoint.eq(proposal.outPoint) && tx.cellDeps[1].depType === 'code'
    && tx.cellDeps[2]?.outPoint.eq(deployment.codes.voteContext) && tx.cellDeps[2].depType === 'code'
    && tx.cellDeps[3]?.outPoint.eq(deployment.codes.vote) && tx.cellDeps[3].depType === 'code', 'dependency_order', 'wallet changed direct vote dependency order');
  checkPreparedWitnesses(original, tx);
  need(fee >= tx.estimateFee(1000n), 'fee', 'vote fee below estimated policy');
  return new PreparedTransaction('vote', signer.client, tx, [funding, deposit, proposal], deployment, refresh);
}
export async function prepareSettlement(client: ccc.Client, proposalPoint: ccc.OutPointLike, recipientLike: ccc.ScriptLike, proofLike: ccc.HexLike, publicLike: ccc.HexLike, refundLike: ccc.ScriptLike, deployment: Deployment, failedFee = 10_000n): Promise<PreparedTransaction> {
  await verifyDeployment(client, deployment);
  const cell = await proposalCell(client, proposalPoint, deployment), params = decodeProposal(cell.outputData);
  const recipient = recipientScript(recipientLike), refund = ccc.Script.from(refundLike);
  need(recipient.hash() === params.recipientHash && same(refund.hash(), cell.cellOutput.lock.args), 'recipient', 'recipient/refund descriptors differ from committed full Script hashes');
  const proof = bytes(proofLike), publicBytes = bytes(publicLike), statement = decodeStatement(publicBytes);
  need(proof.length === 964, 'proof', 'expected release PLONK proof length');
  need(statement.genesis === params.genesis && statement.proposalScript === cell.cellOutput.type!.hash()
    && statement.proposalPoint.eq(cell.outPoint) && statement.proposalDataHash === ccc.hashCkb(cell.outputData),
    'statement', 'proof statement does not bind this actual proposal input');
  need(statement.startNumber <= MAX - BigInt(params.duration) && statement.endNumber === statement.startNumber + BigInt(params.duration)
    && statement.yes <= MAX - statement.no && statement.passed === (statement.yes > statement.no && statement.yes + statement.no >= params.quorum),
    'statement', 'statement window/quorum arithmetic differs from proposal');
  const refresh = async () => {
    const actual = await confirmed(client, cell.outPoint);
    const start = await client.getHeaderByNumberNoCache(statement.startNumber), end = await client.getHeaderByNumberNoCache(statement.endNumber);
    need(actual.blockHash === statement.startHash && actual.blockNumber === statement.startNumber && start?.hash === statement.startHash && end?.hash === statement.endHash,
      'reorg', 'proof anchors or proposal creation association changed; regenerate proof');
  };
  await refresh();
  const data = cat(bytes(cell.outputData), publicBytes);
  const receipt = cell.cellOutput.clone(); receipt.capacity = occupied(receipt, 465);
  need(failedFee >= 0n && failedFee <= 100_000n, 'fee', 'failed refund fee exceeds reserve policy');
  const paid = statement.passed ? params.amount : cell.cellOutput.capacity - receipt.capacity - failedFee;
  need(paid >= params.amount && cell.cellOutput.capacity - receipt.capacity - paid <= 100_000n, 'capacity', 'reserve cannot cover exact receipt and payment/refund');
  const payee = statement.passed ? recipient : refund;
  need(paid >= BigInt(8 + payee.occupiedSize) * SH, 'capacity', 'payment cannot occupy recipient Lock');
  const tx = ccc.Transaction.from({ inputs: [{ previousOutput: cell.outPoint }], outputs: [receipt, { capacity: paid, lock: payee }], outputsData: [data, '0x'],
    cellDeps: ['sp1', 'stateContext', 'proposal', 'treasury'].map(name => dep(deployment, name as keyof typeof pins)),
    headerDeps: [statement.startHash, statement.endHash, statement.genesis] });
  tx.setWitnessArgs(0, { inputType: entry(ccc.hashCkb(publicBytes), recipient), outputType: cat(proof, publicBytes) });
  const fee = cell.cellOutput.capacity - receipt.capacity - paid;
  need(fee >= tx.estimateFee(1000n), 'fee', 'settlement fee below estimated policy');
  return new PreparedTransaction('settle', client, tx, [cell], deployment, refresh);
}

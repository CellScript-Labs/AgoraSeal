//! Sighash-all signing for public disposable-node fixtures only.
//! Message ordering follows ckb-sdk-rust v5.1.0 unlock/signer.rs::generate_message.
//! This module never reads or imports an existing user's private key.
use anyhow::{Result, ensure};
use ckb_types::{bytes::Bytes, core::TransactionView, packed, prelude::*};
use k256::ecdsa::SigningKey;

pub fn fixture_key() -> SigningKey {
    // Public, reproducible, unfunded outside the disposable local chain.
    SigningKey::from_slice(&[0x45; 32]).expect("public fixture scalar")
}
pub fn fixture_args() -> [u8; 20] {
    let key = fixture_key();
    let public = key.verifying_key().to_encoded_point(true);
    agoraseal_protocol::hash(public.as_bytes())[..20]
        .try_into()
        .unwrap()
}

pub fn sign_fixture_group(tx: &TransactionView, indices: &[usize]) -> Result<TransactionView> {
    ensure!(
        !indices.is_empty() && indices.len() <= 128,
        "fixture signing group bound"
    );
    ensure!(
        indices.windows(2).all(|pair| pair[0] < pair[1]),
        "group order"
    );
    ensure!(
        indices.iter().all(|index| *index < tx.inputs().len()),
        "group input missing"
    );
    let mut witnesses: Vec<_> = tx.witnesses().into_iter().collect();
    witnesses.resize_with(witnesses.len().max(tx.inputs().len()), Default::default);
    let raw = witnesses[indices[0]].raw_data();
    let first = if raw.is_empty() {
        packed::WitnessArgs::default()
    } else {
        packed::WitnessArgs::from_slice(&raw)?
    };
    let zero = first
        .clone()
        .as_builder()
        .lock(Some(Bytes::from(vec![0; 65])).pack())
        .build();
    let mut preimage = tx.hash().as_slice().to_vec();
    let mut append = |bytes: &[u8]| {
        preimage.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        preimage.extend_from_slice(bytes);
    };
    append(zero.as_slice());
    for index in indices.iter().skip(1) {
        append(&witnesses[*index].raw_data());
    }
    for witness in &witnesses[tx.inputs().len()..] {
        append(&witness.raw_data());
    }
    let message = agoraseal_protocol::hash(&preimage);
    let (signature, recovery) = fixture_key().sign_prehash_recoverable(&message)?;
    let mut bytes = signature.to_bytes().to_vec();
    bytes.push(recovery.to_byte());
    witnesses[indices[0]] = first
        .as_builder()
        .lock(Some(Bytes::from(bytes)).pack())
        .build()
        .as_bytes()
        .pack();
    Ok(tx.as_advanced_builder().set_witnesses(witnesses).build())
}

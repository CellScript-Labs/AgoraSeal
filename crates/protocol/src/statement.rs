//! Versioned public bytes committed by the proof guest, without host serialization.
use crate::{Error, Hash, OutPoint, Tally, hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicStatement {
    pub genesis: Hash,
    pub proposal_script: Hash,
    pub proposal_outpoint: OutPoint,
    pub proposal_data_hash: Hash,
    pub start_hash: Hash,
    pub end_hash: Hash,
    pub start_number: u64,
    pub end_number: u64,
    pub yes: u64,
    pub no: u64,
    pub counted: u64,
    pub counted_digest: Hash,
    pub passed: bool,
}

impl PublicStatement {
    pub const LEN: usize = 277;

    pub fn from_tally(tally: &Tally) -> Self {
        Self {
            genesis: tally.proposal.genesis,
            proposal_script: tally.proposal_script,
            proposal_outpoint: tally.proposal_outpoint,
            proposal_data_hash: hash(&tally.proposal.encode()),
            start_hash: tally.start_hash,
            end_hash: tally.end_hash,
            start_number: tally.start_number,
            end_number: tally.end_number,
            yes: tally.yes,
            no: tally.no,
            counted: tally.counted,
            counted_digest: tally.counted_digest,
            passed: tally.passed,
        }
    }

    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut bytes = [0; Self::LEN];
        bytes[..8].copy_from_slice(b"AGZKPV01");
        bytes[8..40].copy_from_slice(&self.genesis);
        bytes[40..72].copy_from_slice(&self.proposal_script);
        bytes[72..108].copy_from_slice(&self.proposal_outpoint);
        bytes[108..140].copy_from_slice(&self.proposal_data_hash);
        bytes[140..172].copy_from_slice(&self.start_hash);
        bytes[172..204].copy_from_slice(&self.end_hash);
        for (offset, number) in [
            (204, self.start_number),
            (212, self.end_number),
            (220, self.yes),
            (228, self.no),
            (236, self.counted),
        ] {
            bytes[offset..offset + 8].copy_from_slice(&number.to_le_bytes());
        }
        bytes[244..276].copy_from_slice(&self.counted_digest);
        bytes[276] = u8::from(self.passed);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::LEN || &bytes[..8] != b"AGZKPV01" || bytes[276] > 1 {
            return Err(Error::Encoding);
        }
        let number = |offset| -> Result<u64, Error> {
            Ok(u64::from_le_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| Error::Encoding)?,
            ))
        };
        Ok(Self {
            genesis: bytes[8..40].try_into().map_err(|_| Error::Encoding)?,
            proposal_script: bytes[40..72].try_into().map_err(|_| Error::Encoding)?,
            proposal_outpoint: bytes[72..108].try_into().map_err(|_| Error::Encoding)?,
            proposal_data_hash: bytes[108..140].try_into().map_err(|_| Error::Encoding)?,
            start_hash: bytes[140..172].try_into().map_err(|_| Error::Encoding)?,
            end_hash: bytes[172..204].try_into().map_err(|_| Error::Encoding)?,
            start_number: number(204)?,
            end_number: number(212)?,
            yes: number(220)?,
            no: number(228)?,
            counted: number(236)?,
            counted_digest: bytes[244..276].try_into().map_err(|_| Error::Encoding)?,
            passed: bytes[276] == 1,
        })
    }
}

/// First stdin frame. Followed by exactly `blocks` canonical block frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestInput {
    pub proposal_script: Hash,
    pub start_hash: Hash,
    pub end_hash: Hash,
    pub blocks: u32,
}

impl GuestInput {
    pub const LEN: usize = 108;
    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut bytes = [0; Self::LEN];
        bytes[..8].copy_from_slice(b"AGINPUT1");
        bytes[8..40].copy_from_slice(&self.proposal_script);
        bytes[40..72].copy_from_slice(&self.start_hash);
        bytes[72..104].copy_from_slice(&self.end_hash);
        bytes[104..].copy_from_slice(&self.blocks.to_le_bytes());
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::LEN || &bytes[..8] != b"AGINPUT1" {
            return Err(Error::Encoding);
        }
        Ok(Self {
            proposal_script: bytes[8..40].try_into().map_err(|_| Error::Encoding)?,
            start_hash: bytes[40..72].try_into().map_err(|_| Error::Encoding)?,
            end_hash: bytes[72..104].try_into().map_err(|_| Error::Encoding)?,
            blocks: u32::from_le_bytes(bytes[104..].try_into().map_err(|_| Error::Encoding)?),
        })
    }
}

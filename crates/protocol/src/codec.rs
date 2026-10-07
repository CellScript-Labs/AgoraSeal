use crate::Error;

pub type Hash = [u8; 32];
pub type OutPoint = [u8; 36];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    No = 0,
    Yes = 1,
}

/// Immutable creation data. All integers use little endian; trailing data reject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub genesis: Hash,
    pub vote_code: Hash,
    /// Complete Nervos DAO Script hash; deployment fixes the accepted network script.
    pub dao_script: Hash,
    pub duration: u32,
    pub quorum: u64,
    pub amount: u64,
    pub recipient: Hash,
    pub description: Hash,
}

impl Proposal {
    pub const LEN: usize = 188;
    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut bytes = [0; Self::LEN];
        bytes[..8].copy_from_slice(b"AGPROP01");
        bytes[8..40].copy_from_slice(&self.genesis);
        bytes[40..72].copy_from_slice(&self.vote_code);
        bytes[72..76].copy_from_slice(&self.duration.to_le_bytes());
        bytes[76..84].copy_from_slice(&self.quorum.to_le_bytes());
        bytes[84..92].copy_from_slice(&self.amount.to_le_bytes());
        bytes[92..124].copy_from_slice(&self.recipient);
        bytes[124..156].copy_from_slice(&self.description);
        bytes[156..188].copy_from_slice(&self.dao_script);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::LEN || &bytes[..8] != b"AGPROP01" {
            return Err(Error::Encoding);
        }
        let value = Self {
            genesis: bytes[8..40].try_into().map_err(|_| Error::Encoding)?,
            vote_code: bytes[40..72].try_into().map_err(|_| Error::Encoding)?,
            duration: u32::from_le_bytes(bytes[72..76].try_into().map_err(|_| Error::Encoding)?),
            quorum: u64::from_le_bytes(bytes[76..84].try_into().map_err(|_| Error::Encoding)?),
            amount: u64::from_le_bytes(bytes[84..92].try_into().map_err(|_| Error::Encoding)?),
            recipient: bytes[92..124].try_into().map_err(|_| Error::Encoding)?,
            description: bytes[124..156].try_into().map_err(|_| Error::Encoding)?,
            dao_script: bytes[156..188].try_into().map_err(|_| Error::Encoding)?,
        };
        if value.duration == 0 || value.quorum == 0 || value.amount == 0 {
            return Err(Error::Policy);
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ballot {
    pub deposit: OutPoint,
    pub weight: u64,
    pub choice: Choice,
}

impl Ballot {
    pub const LEN: usize = 53;
    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut bytes = [0; Self::LEN];
        bytes[..8].copy_from_slice(b"AGVOTE01");
        bytes[8..44].copy_from_slice(&self.deposit);
        bytes[44..52].copy_from_slice(&self.weight.to_le_bytes());
        bytes[52] = self.choice as u8;
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::LEN || &bytes[..8] != b"AGVOTE01" {
            return Err(Error::Encoding);
        }
        let choice = match bytes[52] {
            0 => Choice::No,
            1 => Choice::Yes,
            _ => return Err(Error::Encoding),
        };
        let weight = u64::from_le_bytes(bytes[44..52].try_into().map_err(|_| Error::Encoding)?);
        if weight == 0 {
            return Err(Error::Policy);
        }
        Ok(Self {
            deposit: bytes[8..44].try_into().map_err(|_| Error::Encoding)?,
            weight,
            choice,
        })
    }
}

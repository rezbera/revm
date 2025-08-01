use core::fmt;
use primitives::{b256, bytes, Address, Bytes, B256};

/// Hash of EF01 bytes that is used for EXTCODEHASH when called from legacy bytecode.
pub const EIP7702_MAGIC_HASH: B256 =
    b256!("0xeadcdba66a79ab5dce91622d1d75c8cff5cff0b96944c3bf1072cd08ce018329");

/// EIP-7702 Version Magic in u16 form
pub const EIP7702_MAGIC: u16 = 0xEF01;

/// EIP-7702 magic number in array form
pub static EIP7702_MAGIC_BYTES: Bytes = bytes!("ef01");

/// EIP-7702 first version of bytecode
pub const EIP7702_VERSION: u8 = 0;

/// Bytecode of delegated account, specified in EIP-7702
///
/// Format of EIP-7702 bytecode consist of:
/// `0xEF01` (MAGIC) + `0x00` (VERSION) + 20 bytes of address.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Eip7702Bytecode {
    /// Address of the delegated account.
    pub delegated_address: Address,
    /// Version of the EIP-7702 bytecode. Currently only version 0 is supported.
    pub version: u8,
    /// Raw bytecode.
    pub raw: Bytes,
}

impl Eip7702Bytecode {
    /// Creates a new EIP-7702 bytecode or returns None if the raw bytecode is invalid.
    #[inline]
    pub fn new_raw(raw: Bytes) -> Result<Self, Eip7702DecodeError> {
        if raw.len() != 23 {
            return Err(Eip7702DecodeError::InvalidLength);
        }
        if !raw.starts_with(&EIP7702_MAGIC_BYTES) {
            return Err(Eip7702DecodeError::InvalidMagic);
        }

        // Only supported version is version 0.
        if raw[2] != EIP7702_VERSION {
            return Err(Eip7702DecodeError::UnsupportedVersion);
        }

        Ok(Self {
            delegated_address: Address::new(raw[3..].try_into().unwrap()),
            version: raw[2],
            raw,
        })
    }

    /// Creates a new EIP-7702 bytecode with the given address.
    pub fn new(address: Address) -> Self {
        let mut raw = EIP7702_MAGIC_BYTES.to_vec();
        raw.push(EIP7702_VERSION);
        raw.extend(&address);
        Self {
            delegated_address: address,
            version: EIP7702_VERSION,
            raw: raw.into(),
        }
    }

    /// Returns the raw bytecode with version MAGIC number.
    #[inline]
    pub fn raw(&self) -> &Bytes {
        &self.raw
    }

    /// Returns the address of the delegated contract.
    #[inline]
    pub fn address(&self) -> Address {
        self.delegated_address
    }

    /// Returns the EIP7702 version of the delegated contract.
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }
}

/// Bytecode errors
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Eip7702DecodeError {
    /// Invalid length of the raw bytecode
    ///
    /// It should be 23 bytes.
    InvalidLength,
    /// Invalid magic number
    ///
    /// All Eip7702 bytecodes should start with the magic number 0xEF01.
    InvalidMagic,
    /// Unsupported version
    ///
    /// Only supported version is version 0x00
    UnsupportedVersion,
}

impl fmt::Display for Eip7702DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::InvalidLength => "Eip7702 is not 23 bytes long",
            Self::InvalidMagic => "Bytecode is not starting with 0xEF01",
            Self::UnsupportedVersion => "Unsupported Eip7702 version.",
        };
        f.write_str(s)
    }
}

impl core::error::Error for Eip7702DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_decode() {
        let raw = bytes!("ef01deadbeef");
        assert_eq!(
            Eip7702Bytecode::new_raw(raw),
            Err(Eip7702DecodeError::InvalidLength)
        );

        let raw = bytes!("ef0101deadbeef00000000000000000000000000000000");
        assert_eq!(
            Eip7702Bytecode::new_raw(raw),
            Err(Eip7702DecodeError::UnsupportedVersion)
        );

        let raw = bytes!("ef0100deadbeef00000000000000000000000000000000");
        let address = raw[3..].try_into().unwrap();
        assert_eq!(
            Eip7702Bytecode::new_raw(raw.clone()),
            Ok(Eip7702Bytecode {
                delegated_address: address,
                version: 0,
                raw,
            })
        );
    }

    #[test]
    fn create_eip7702_bytecode_from_address() {
        let address = Address::new([0x01; 20]);
        let bytecode = Eip7702Bytecode::new(address);
        assert_eq!(bytecode.delegated_address, address);
        assert_eq!(
            bytecode.raw,
            bytes!("ef01000101010101010101010101010101010101010101")
        );
    }
}

//! # RIP-7212 secp256r1 Precompile
//!
//! This module implements the [RIP-7212](https://github.com/ethereum/RIPs/blob/master/RIPS/rip-7212.md) precompile for
//! secp256r1 curve support.
//!
//! The main purpose of this precompile is to verify ECDSA signatures that use the secp256r1, or
//! P256 elliptic curve. The [`P256VERIFY`] const represents the implementation of this precompile,
//! with the address that it is currently deployed at.
use crate::{
    crypto, u64_to_address, PrecompileError, PrecompileOutput, PrecompileResult,
    PrecompileWithAddress,
};
use p256::{
    ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey},
    EncodedPoint,
};
use primitives::{alloy_primitives::B512, Bytes, B256};

/// Address of secp256r1 precompile.
pub const P256VERIFY_ADDRESS: u64 = 256;

/// Base gas fee for secp256r1 p256verify operation.
pub const P256VERIFY_BASE_GAS_FEE: u64 = 3450;

/// Base gas fee for secp256r1 p256verify operation post Osaka.
pub const P256VERIFY_BASE_GAS_FEE_OSAKA: u64 = 6900;

/// Returns the secp256r1 precompile with its address.
pub fn precompiles() -> impl Iterator<Item = PrecompileWithAddress> {
    [P256VERIFY].into_iter()
}

/// [RIP-7212](https://github.com/ethereum/RIPs/blob/master/RIPS/rip-7212.md#specification) secp256r1 precompile.
pub const P256VERIFY: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(P256VERIFY_ADDRESS), p256_verify);

/// [RIP-7212](https://github.com/ethereum/RIPs/blob/master/RIPS/rip-7212.md#specification) secp256r1 precompile.
pub const P256VERIFY_OSAKA: PrecompileWithAddress =
    PrecompileWithAddress(u64_to_address(P256VERIFY_ADDRESS), p256_verify_osaka);

/// secp256r1 precompile logic. It takes the input bytes sent to the precompile
/// and the gas limit. The output represents the result of verifying the
/// secp256r1 signature of the input.
///
/// The input is encoded as follows:
///
/// | signed message hash |  r  |  s  | public key x | public key y |
/// | :-----------------: | :-: | :-: | :----------: | :----------: |
/// |          32         | 32  | 32  |     32       |      32      |
pub fn p256_verify(input: &[u8], gas_limit: u64) -> PrecompileResult {
    p256_verify_inner(input, gas_limit, P256VERIFY_BASE_GAS_FEE)
}

/// secp256r1 precompile logic with Osaka gas cost. It takes the input bytes sent to the precompile
/// and the gas limit. The output represents the result of verifying the
/// secp256r1 signature of the input.
///
/// The input is encoded as follows:
///
/// | signed message hash |  r  |  s  | public key x | public key y |
/// | :-----------------: | :-: | :-: | :----------: | :----------: |
/// |          32         | 32  | 32  |     32       |      32      |
pub fn p256_verify_osaka(input: &[u8], gas_limit: u64) -> PrecompileResult {
    p256_verify_inner(input, gas_limit, P256VERIFY_BASE_GAS_FEE_OSAKA)
}

fn p256_verify_inner(input: &[u8], gas_limit: u64, gas_cost: u64) -> PrecompileResult {
    if gas_cost > gas_limit {
        return Err(PrecompileError::OutOfGas);
    }
    let result = if verify_impl(input) {
        B256::with_last_byte(1).into()
    } else {
        Bytes::new()
    };
    Ok(PrecompileOutput::new(gas_cost, result))
}

/// Returns `Some(())` if the signature included in the input byte slice is
/// valid, `None` otherwise.
pub fn verify_impl(input: &[u8]) -> bool {
    if input.len() != 160 {
        return false;
    }

    // msg signed (msg is already the hash of the original message)
    let msg = <&B256>::try_from(&input[..32]).unwrap();
    // r, s: signature
    let sig = <&B512>::try_from(&input[32..96]).unwrap();
    // x, y: public key
    let pk = <&B512>::try_from(&input[96..160]).unwrap();

    crypto().secp256r1_verify_signature(&msg.0, &sig.0, &pk.0)
}

pub(crate) fn verify_signature(msg: [u8; 32], sig: [u8; 64], pk: [u8; 64]) -> Option<()> {
    // Can fail only if the input is not exact length.
    let signature = Signature::from_slice(&sig).ok()?;
    // Decode the public key bytes (x,y coordinates) using EncodedPoint
    let encoded_point = EncodedPoint::from_untagged_bytes(&pk.into());
    // Create VerifyingKey from the encoded point
    let public_key = VerifyingKey::from_encoded_point(&encoded_point).ok()?;

    public_key.verify_prehash(&msg, &signature).ok()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::PrecompileError;
    use primitives::hex::FromHex;
    use rstest::rstest;

    #[rstest]
    // Test vectors from https://github.com/daimo-eth/p256-verifier/tree/master/test-vectors
    #[case::ok_1("4cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4da73bd4903f0ce3b639bbbf6e8e80d16931ff4bcf5993d58468e8fb19086e8cac36dbcd03009df8c59286b162af3bd7fcc0450c9aa81be5d10d312af6c66b1d604aebd3099c618202fcfe16ae7770b0c49ab5eadf74b754204a3bb6060e44eff37618b065f9832de4ca6ca971a7a1adc826d0f7c00181a5fb2ddf79ae00b4e10e", true)]
    #[case::ok_2("3fec5769b5cf4e310a7d150508e82fb8e3eda1c2c94c61492d3bd8aea99e06c9e22466e928fdccef0de49e3503d2657d00494a00e764fd437bdafa05f5922b1fbbb77c6817ccf50748419477e843d5bac67e6a70e97dde5a57e0c983b777e1ad31a80482dadf89de6302b1988c82c29544c9c07bb910596158f6062517eb089a2f54c9a0f348752950094d3228d3b940258c75fe2a413cb70baa21dc2e352fc5", true)]
    #[case::ok_3("e775723953ead4a90411a02908fd1a629db584bc600664c609061f221ef6bf7c440066c8626b49daaa7bf2bcc0b74be4f7a1e3dcf0e869f1542fe821498cbf2de73ad398194129f635de4424a07ca715838aefe8fe69d1a391cfa70470795a80dd056866e6e1125aff94413921880c437c9e2570a28ced7267c8beef7e9b2d8d1547d76dfcf4bee592f5fefe10ddfb6aeb0991c5b9dbbee6ec80d11b17c0eb1a", true)]
    #[case::ok_4("b5a77e7a90aa14e0bf5f337f06f597148676424fae26e175c6e5621c34351955289f319789da424845c9eac935245fcddd805950e2f02506d09be7e411199556d262144475b1fa46ad85250728c600c53dfd10f8b3f4adf140e27241aec3c2da3a81046703fccf468b48b145f939efdbb96c3786db712b3113bb2488ef286cdcef8afe82d200a5bb36b5462166e8ce77f2d831a52ef2135b2af188110beaefb1", true)]
    #[case::ok_5("858b991cfd78f16537fe6d1f4afd10273384db08bdfc843562a22b0626766686f6aec8247599f40bfe01bec0e0ecf17b4319559022d4d9bf007fe929943004eb4866760dedf31b7c691f5ce665f8aae0bda895c23595c834fecc2390a5bcc203b04afcacbb4280713287a2d0c37e23f7513fab898f2c1fefa00ec09a924c335d9b629f1d4fb71901c3e59611afbfea354d101324e894c788d1c01f00b3c251b2", true)]
    #[case::fail_wrong_msg_1("3cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4da73bd4903f0ce3b639bbbf6e8e80d16931ff4bcf5993d58468e8fb19086e8cac36dbcd03009df8c59286b162af3bd7fcc0450c9aa81be5d10d312af6c66b1d604aebd3099c618202fcfe16ae7770b0c49ab5eadf74b754204a3bb6060e44eff37618b065f9832de4ca6ca971a7a1adc826d0f7c00181a5fb2ddf79ae00b4e10e", false)]
    #[case::fail_wrong_msg_2("afec5769b5cf4e310a7d150508e82fb8e3eda1c2c94c61492d3bd8aea99e06c9e22466e928fdccef0de49e3503d2657d00494a00e764fd437bdafa05f5922b1fbbb77c6817ccf50748419477e843d5bac67e6a70e97dde5a57e0c983b777e1ad31a80482dadf89de6302b1988c82c29544c9c07bb910596158f6062517eb089a2f54c9a0f348752950094d3228d3b940258c75fe2a413cb70baa21dc2e352fc5", false)]
    #[case::fail_wrong_msg_3("f775723953ead4a90411a02908fd1a629db584bc600664c609061f221ef6bf7c440066c8626b49daaa7bf2bcc0b74be4f7a1e3dcf0e869f1542fe821498cbf2de73ad398194129f635de4424a07ca715838aefe8fe69d1a391cfa70470795a80dd056866e6e1125aff94413921880c437c9e2570a28ced7267c8beef7e9b2d8d1547d76dfcf4bee592f5fefe10ddfb6aeb0991c5b9dbbee6ec80d11b17c0eb1a", false)]
    #[case::fail_wrong_msg_4("c5a77e7a90aa14e0bf5f337f06f597148676424fae26e175c6e5621c34351955289f319789da424845c9eac935245fcddd805950e2f02506d09be7e411199556d262144475b1fa46ad85250728c600c53dfd10f8b3f4adf140e27241aec3c2da3a81046703fccf468b48b145f939efdbb96c3786db712b3113bb2488ef286cdcef8afe82d200a5bb36b5462166e8ce77f2d831a52ef2135b2af188110beaefb1", false)]
    #[case::fail_wrong_msg_5("958b991cfd78f16537fe6d1f4afd10273384db08bdfc843562a22b0626766686f6aec8247599f40bfe01bec0e0ecf17b4319559022d4d9bf007fe929943004eb4866760dedf31b7c691f5ce665f8aae0bda895c23595c834fecc2390a5bcc203b04afcacbb4280713287a2d0c37e23f7513fab898f2c1fefa00ec09a924c335d9b629f1d4fb71901c3e59611afbfea354d101324e894c788d1c01f00b3c251b2", false)]
    #[case::fail_short_input_1("4cee90eb86eaa050036147a12d49004b6a", false)]
    #[case::fail_short_input_2("4cee90eb86eaa050036147a12d49004b6a958b991cfd78f16537fe6d1f4afd10273384db08bdfc843562a22b0626766686f6aec8247599f40bfe01bec0e0ecf17b4319559022d4d9bf007fe929943004eb4866760dedf319", false)]
    #[case::fail_long_input("4cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4da73bd4903f0ce3b639bbbf6e8e80d16931ff4bcf5993d58468e8fb19086e8cac36dbcd03009df8c59286b162af3bd7fcc0450c9aa81be5d10d312af6c66b1d604aebd3099c618202fcfe16ae7770b0c49ab5eadf74b754204a3bb6060e44eff37618b065f9832de4ca6ca971a7a1adc826d0f7c00181a5fb2ddf79ae00b4e10e00", false)]
    #[case::fail_invalid_sig("4cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4dffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff4aebd3099c618202fcfe16ae7770b0c49ab5eadf74b754204a3bb6060e44eff37618b065f9832de4ca6ca971a7a1adc826d0f7c00181a5fb2ddf79ae00b4e10e", false)]
    #[case::fail_invalid_pubkey("4cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4da73bd4903f0ce3b639bbbf6e8e80d16931ff4bcf5993d58468e8fb19086e8cac36dbcd03009df8c59286b162af3bd7fcc0450c9aa81be5d10d312af6c66b1d6000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", false)]
    fn test_sig_verify(#[case] input: &str, #[case] expect_success: bool) {
        let input = Bytes::from_hex(input).unwrap();
        let target_gas = 3_500u64;
        let outcome = p256_verify(&input, target_gas).unwrap();
        assert_eq!(outcome.gas_used, 3_450u64);
        let expected_result = if expect_success {
            B256::with_last_byte(1).into()
        } else {
            Bytes::new()
        };
        assert_eq!(outcome.bytes, expected_result);
    }

    #[rstest]
    fn test_not_enough_gas_errors() {
        let input = Bytes::from_hex("4cee90eb86eaa050036147a12d49004b6b9c72bd725d39d4785011fe190f0b4da73bd4903f0ce3b639bbbf6e8e80d16931ff4bcf5993d58468e8fb19086e8cac36dbcd03009df8c59286b162af3bd7fcc0450c9aa81be5d10d312af6c66b1d604aebd3099c618202fcfe16ae7770b0c49ab5eadf74b754204a3bb6060e44eff37618b065f9832de4ca6ca971a7a1adc826d0f7c00181a5fb2ddf79ae00b4e10e").unwrap();
        let target_gas = 2_500u64;
        let result = p256_verify(&input, target_gas);

        assert!(result.is_err());
        assert_eq!(result.err(), Some(PrecompileError::OutOfGas));
    }

    #[rstest]
    #[case::ok_1("b5a77e7a90aa14e0bf5f337f06f597148676424fae26e175c6e5621c34351955289f319789da424845c9eac935245fcddd805950e2f02506d09be7e411199556d262144475b1fa46ad85250728c600c53dfd10f8b3f4adf140e27241aec3c2da3a81046703fccf468b48b145f939efdbb96c3786db712b3113bb2488ef286cdcef8afe82d200a5bb36b5462166e8ce77f2d831a52ef2135b2af188110beaefb1", true)]
    #[case::fail_1("b5a77e7a90aa14e0bf5f337f06f597148676424fae26e175c6e5621c34351955289f319789da424845c9eac935245fcddd805950e2f02506d09be7e411199556d262144475b1fa46ad85250728c600c53dfd10f8b3f4adf140e27241aec3c2daaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaef8afe82d200a5bb36b5462166e8ce77f2d831a52ef2135b2af188110beaefb1", false)]
    fn test_verify_impl(#[case] input: &str, #[case] expect_success: bool) {
        let input = Bytes::from_hex(input).unwrap();
        let result = verify_impl(&input);

        assert_eq!(result, expect_success);
    }
}

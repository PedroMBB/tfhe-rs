use crate::conformance::ParameterSetConformant;
use crate::integer::BooleanBlock;
use crate::named::Named;
use crate::prelude::FheTryEncrypt;
use crate::shortint::parameters::CiphertextConformanceParams;
use crate::shortint::CompressedCiphertext;
use crate::{ClientKey, FheBool};
use serde::{Deserialize, Serialize};

/// Compressed [FheBool]
///
/// Meant to save in storage space / transfer.
///
/// - A Compressed type must be decompressed before it can be used.
/// - It is not possible to compress an existing [FheBool], compression can only be achieved at
///   encryption time
///
/// # Example
///
/// ```
/// use tfhe::prelude::*;
/// use tfhe::{generate_keys, CompressedFheBool, ConfigBuilder};
///
/// let (client_key, _) = generate_keys(ConfigBuilder::default());
/// let compressed = CompressedFheBool::encrypt(true, &client_key);
///
/// let decompressed = compressed.decompress();
/// let decrypted: bool = decompressed.decrypt(&client_key);
/// assert_eq!(decrypted, true);
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct CompressedFheBool {
    pub(in crate::high_level_api) ciphertext: CompressedCiphertext,
}

impl CompressedFheBool {
    pub(in crate::high_level_api) fn new(ciphertext: CompressedCiphertext) -> Self {
        Self { ciphertext }
    }

    /// Decompresses itself into a [FheBool]
    ///
    /// See [CompressedFheBool] example.
    pub fn decompress(&self) -> FheBool {
        FheBool::new(BooleanBlock::new_unchecked(
            self.ciphertext.clone().decompress(),
        ))
    }
}

impl FheTryEncrypt<bool, ClientKey> for CompressedFheBool {
    type Error = crate::high_level_api::errors::Error;

    /// Creates a compressed encryption of a boolean value
    fn try_encrypt(value: bool, key: &ClientKey) -> Result<Self, Self::Error> {
        let ciphertext = key.key.key.key.encrypt_compressed(u64::from(value));
        Ok(Self::new(ciphertext))
    }
}

impl ParameterSetConformant for CompressedFheBool {
    type ParameterSet = CiphertextConformanceParams;
    fn is_conformant(&self, params: &CiphertextConformanceParams) -> bool {
        self.ciphertext.is_conformant(params)
    }
}

impl Named for CompressedFheBool {
    const NAME: &'static str = "high_level_api::CompressedFheBool";
}

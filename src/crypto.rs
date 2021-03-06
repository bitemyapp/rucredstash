use aes_ctr::stream_cipher::generic_array::GenericArray;
use aes_ctr::stream_cipher::{NewStreamCipher, SyncStreamCipher};
use aes_ctr::Aes256Ctr;
use ring::hmac;

pub struct Crypto {
    default_nonce: [u8; 16],
}

impl Crypto {
    pub fn new() -> Self {
        Crypto {
            default_nonce: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        }
    }

    pub fn verify_ciphertext_integrity(
        hmac_key: &hmac::Key,
        ciphertext: &Vec<u8>,
        hmac: &Vec<u8>,
    ) -> bool {
        match hmac::verify(&hmac_key, ciphertext.as_ref(), hmac.as_ref()) {
            Ok(()) => true,
            Err(_) => false,
        }
    }

    pub fn aes_encrypt_ctr(self, plaintext: Vec<u8>, key: bytes::Bytes) -> Vec<u8> {
        // credstash uses AES symmetric encryption in CTR mode.
        // The key size used is 32 bytes (256 bits).
        let cipher_key: &GenericArray<u8, _> = GenericArray::from_slice(&key);
        let nonce: &GenericArray<u8, _> = GenericArray::from_slice(&self.default_nonce);
        let mut cipher = Aes256Ctr::new(&cipher_key, &nonce);
        let mut c1 = plaintext.clone();
        let f: &mut [u8] = {
            let c2: &mut [u8] = c1.as_mut();
            cipher.apply_keystream(c2);
            c2
        };
        f.to_vec()
    }

    pub fn aes_decrypt_ctr(self, ciphertext: Vec<u8>, key: Vec<u8>) -> Vec<u8> {
        let cipher_key: &GenericArray<u8, _> = GenericArray::from_slice(&key[0..]);
        let nonce: &GenericArray<u8, _> = GenericArray::from_slice(&self.default_nonce);
        let mut cipher = Aes256Ctr::new(&cipher_key, &nonce);
        let mut c1 = ciphertext.clone();
        let f: &mut [u8] = {
            let c2: &mut [u8] = c1.as_mut();
            cipher.apply_keystream(c2);
            c2
        };
        f.to_vec()
    }
}

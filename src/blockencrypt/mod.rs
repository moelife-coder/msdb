use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::secretbox;
pub fn password_deriv(password: &str, salt: pwhash::Salt) -> secretbox::Key {
    let mut k = secretbox::Key([0; secretbox::KEYBYTES]);
    {
        let secretbox::Key(ref mut kb) = k;
        pwhash::derive_key(
            kb,
            password.as_bytes(),
            &salt,
            pwhash::OPSLIMIT_INTERACTIVE,
            pwhash::MEMLIMIT_INTERACTIVE,
        )
        .unwrap();
    }
    k
}
pub fn encrypt_block(input_block: &[u8], password: &secretbox::Key) -> (Vec<u8>, secretbox::Nonce) {
    let nonce = secretbox::gen_nonce();
    let plaintext = &input_block[..];
    (secretbox::seal(plaintext, &nonce, password), nonce)
    //let their_plaintext = secretbox::open(&ciphertext, &nonce, &key).unwrap();
    //assert!(plaintext == &their_plaintext[..]);
}
pub fn decrypt_block(
    input_block: &[u8],
    password: &secretbox::Key,
    nonce: secretbox::Nonce,
) -> Vec<u8> {
    secretbox::open(input_block, &nonce, password).unwrap()
}

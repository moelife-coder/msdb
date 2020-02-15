use sodiumoxide::crypto::secretbox;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
pub fn read_with_nonce(filename: &str) -> (Vec<u8>, secretbox::Nonce) {
    (read_all(filename), read_nonce(filename))
}
pub fn write_with_nonce(filename: &str, data: &[u8], nonce: secretbox::Nonce) {
    let mut nonce_filename = filename.to_string();
    nonce_filename.push_str(".nonce");
    write_all(filename, data);
    write_all(&nonce_filename[..], &nonce[..]);
}
/// Write data to a file.
///
/// Only use this function for writing salt, since there's no point for using it elsewhere.
pub fn write_all(filename: &str, data: &[u8]) {
    let mut file = File::create(filename).unwrap();
    file.write_all(data).expect("Unable to write to file");
}
/// Read a file to a Vec<u8>.
///
/// Only use this function for reading salt, since there's no point for using it elsewhere.
pub fn read_all(filename: &str) -> Vec<u8> {
    let file = File::open(filename).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut contents: Vec<u8> = Vec::new();
    buf_reader.read_to_end(&mut contents).unwrap();
    contents
}
fn read_nonce(filename: &str) -> secretbox::Nonce {
    let mut filename = filename.to_string();
    filename.push_str(".nonce");
    let mut nonce_file = File::open(filename).expect("Error when opening nonce file");
    let mut nonce_vec = Vec::new();
    nonce_file
        .read_to_end(&mut nonce_vec)
        .expect("Error when reading nonce file");
    secretbox::Nonce::from_slice(&nonce_vec[..]).unwrap()
}

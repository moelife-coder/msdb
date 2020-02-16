use super::binary_io;
use super::blockencrypt;
use super::metadata;
use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::secretbox;
const DENY_ACCESS_FOR_UNMATCH_VERSION: bool = true;
/// Create a database
pub fn new_database(database_name: &str, password: &str, database_version_code: u8) {
    std::fs::create_dir_all(&database_name).expect("Unable to create database root directory");
    let salt = pwhash::gen_salt();
    let salt_directory = format!("{}/salt", database_name);
    binary_io::write_all(&salt_directory, &salt[..].to_vec());
    let mut initial_metadata = metadata::Metadata::create();
    initial_metadata.new_attribute(&String::from("ver"), &format!("{}", database_version_code));
    initial_metadata.new_attribute(&String::from("type"), &String::from("msdb"));
    let metadata_filename = format!("{}/metadata", database_name);
    let data = blockencrypt::encrypt_block(
        &initial_metadata.to_vec(),
        &blockencrypt::password_deriv(password, salt),
    );
    binary_io::write_with_nonce(&metadata_filename, &data.0, data.1);
    println!("Completed. Have a nice day.");
}
pub fn select_database(
    database_name: &str,
    password: &secretbox::Key,
    database_version_code: u8,
) -> metadata::Metadata {
    let mut result_metadata = metadata::Metadata::create();
    let block_decrypted = {
        let block_directory = format!("{}/metadata", database_name);
        let block = binary_io::read_with_nonce(&block_directory);
        blockencrypt::decrypt_block(&block.0, password, block.1)
    };
    result_metadata.import(block_decrypted);
    if result_metadata
        .attribute()
        .get(&"type".to_string())
        .expect("Unable to identify metadata type")
        .as_str()
        != "msdb"
    {
        panic!("Unexpected metadata type");
    }
    {
        let database_version: u8 = result_metadata
            .attribute()
            .get(&"ver".to_string())
            .expect("Unable to fetch database version from metadata")
            .parse()
            .expect("Unable to translate database version from metadata to integer");
        if database_version != database_version_code {
            if DENY_ACCESS_FOR_UNMATCH_VERSION {
                panic!("Database version({}) is not equal than database manager version({}). Denying access.", database_version, database_version_code);
            } else {
                println!(
                    "Warning: Database version({}) is not equal than database manager version({})",
                    database_version, database_version_code
                );
            }
        }
    }
    result_metadata
}
pub fn exit() -> ! {
    std::process::exit(0);
}

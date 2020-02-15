use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::secretbox;
use std::collections::HashMap;
use std::path;
mod binary_io;
mod blockencrypt;
mod blocks;
mod db_commands;
mod metadata;
//FIXME: 输入database来创建有bug(需要去掉\n)
/// A breief changelog
/// Version 1: initial release
/// Version 2: Completely rewritten `db_command`
/// Version 3: Rewritten `block io`
const VERSION_NUMBER: u8 = 3;
fn main() {
    //let mut database_root = db_commands::DatabaseLocation::new();
    let mut current_location = db_commands::DatabaseLocation::new();
    let mut password: (secretbox::Key, bool) = (secretbox::gen_key(), false);
    //(secretbox::gen_key(), false);
    let mut main_metadata: metadata::Metadata = metadata::Metadata::create();
    //let mut metadata_index: HashMap<[u8; db_commands::METADATA_INDEX_LEN as usize],metadata::Metadata> = HashMap::new();
    let mut structure_cache: HashMap<
        [u8; blocks::CELL_IDENTIFIER_LENGTH as usize],
        db_commands::Structure,
    > = HashMap::new();
    //let mut block_queue: HashMap<[u8; db_commands::METADATA_INDEX_LEN as usize],blocks::BlockQueue> = HashMap::new();
    sodiumoxide::init().expect("Unable to initialize SoldiumMoxide");
    let mut rl = rustyline::Editor::<()>::new();
    loop {
        let user_input = {
            let p = format!("{} >", current_location);
            let readline = rl.readline(&p);
            match readline {
                Ok(line) => line,
                Err(rustyline::error::ReadlineError::Interrupted)
                | Err(rustyline::error::ReadlineError::Eof) => std::process::exit(0),
                _ => String::new(),
            }
        };
        let user_command_unparsed = user_input.to_ascii_lowercase();
        let user_command = user_command_unparsed.split_whitespace().next();
        let mut parsed_command = {
            let mut result = user_input.split_whitespace();
            result.next();
            result
        };
        if let Some(command) = user_command {
            match command {
                "create" => {
                    println!("---New Database Wizard---");
                    let directory = if let Some(k) = parsed_command.next() {
                        k.to_string()
                    } else {
                        let mut database_directory_ok: bool = false;
                        let mut directory = String::new();
                        while !database_directory_ok {
                            let p = format!("Database directory:(relative) ");
                            let readline = rl.readline(&p);
                            match readline {
                                Ok(line) => directory = line,
                                _ => {}
                            };
                            if !path::Path::new(&directory).is_relative() {
                                println!("Please enter a relative path");
                            } else if path::Path::new(&directory).exists() {
                                println!("Directory {} already exists", directory);
                            } else {
                                database_directory_ok = true;
                            };
                        }
                        directory
                    };
                    std::fs::create_dir_all(&directory)
                        .expect("Unable to create database root directory");
                    let password = rpassword::prompt_password_stdout("Password: ")
                        .expect("Unable to read password using rpassword");
                    {
                        let mut password_comfirm = String::new();
                        while password != password_comfirm {
                            password_comfirm =
                                rpassword::prompt_password_stdout("Confirm password: ")
                                    .expect("Unable to read password confirmation using rpassword");
                        }
                    }
                    {
                        let salt = pwhash::gen_salt();
                        let salt_directory = format!("{}/salt", directory);
                        binary_io::write_all(&salt_directory, &salt[..].to_vec());
                        let mut initial_metadata = metadata::Metadata::create();
                        initial_metadata.new_attribute(&String::from("ver"), &String::from("3"));
                        initial_metadata
                            .new_attribute(&String::from("type"), &String::from("msdb"));
                        let metadata_filename = format!("{}/metadata", directory);
                        let data = blockencrypt::encrypt_block(
                            &initial_metadata.to_vec(),
                            &blockencrypt::password_deriv(&password, salt),
                        );
                        binary_io::write_with_nonce(&metadata_filename, &data.0, data.1);
                    };
                    println!("Completed. Have a nice day.");
                }
                "decrypt" => {
                    let password_raw: String;
                    if password.1 {
                        println!("The database has already been unlocked.");
                    } else {
                        current_location.select_root(if let Some(i) = parsed_command.next() {
                            i.to_string()
                        } else {
                            let p = format!("Database directory:");
                            let mut directory = String::new();
                            let readline = rl.readline(&p);
                            match readline {
                                Ok(line) => directory = line,
                                _ => {}
                            };
                            directory
                        });
                        println!(
                            "Selecting database {}",
                            current_location.root_folder().unwrap()
                        );
                        password = (
                            {
                                password_raw = match parsed_command.next() {
                                    Some(i) => i.to_string(),
                                    None => {
                                        rpassword::prompt_password_stdout("Password: ").unwrap()
                                    }
                                };
                                let salt = {
                                    let salt_directory =
                                        format!("{}/salt", current_location.root_folder().unwrap());
                                    let salt_vec = binary_io::read_all(&salt_directory);
                                    pwhash::Salt::from_slice(&salt_vec[..]).unwrap()
                                };
                                blockencrypt::password_deriv(&password_raw, salt)
                            },
                            true,
                        );
                        //Read metadata and check if it's right
                        {
                            let block_decrypted = {
                                let block_directory =
                                    format!("{}/metadata", current_location.root_folder().unwrap());
                                let block = binary_io::read_with_nonce(&block_directory);
                                blockencrypt::decrypt_block(&block.0, &password.0, block.1)
                            };
                            main_metadata.import(block_decrypted);
                        };
                        if main_metadata
                            .attribute()
                            .get(&"type".to_string())
                            .expect("Unable to identify metadata type")
                            .as_str()
                            != "msdb"
                        {
                            panic!("Unexpected metadata type");
                        }
                        {
                            let database_version: u8 = main_metadata
                                .attribute()
                                .get(&"ver".to_string())
                                .expect("Unable to fetch database version from metadata")
                                .parse()
                                .expect(
                                    "Unable to translate database version from metadata to integer",
                                );
                            if database_version != VERSION_NUMBER {
                                panic!("Database syntax version({}) is not equal than database manager version({}). Corwardly denying access.", database_version, VERSION_NUMBER);
                            }
                            println!("Database Version: {}", database_version);
                        }
                    }
                }
                "raw_meta" => {
                    if password.1 {
                        println!("{}", main_metadata);
                    } else {
                        panic!("You are not logged in.");
                    }
                }
                "logout" => {
                    if password.1 {
                        main_metadata.clear();
                        password = (secretbox::gen_key(), false);
                        current_location.logout();
                    } else {
                        panic!("You are not logged in.");
                    }
                }
                "exit" => std::process::exit(0),
                _ => {
                    if password.1 {
                        db_commands::run_commands(
                            &user_command_unparsed,
                            &mut main_metadata,
                            &mut current_location,
                            &password.0,
                            &mut structure_cache,
                        );
                    } else {
                        panic!("Unknown command {}", user_input)
                    }
                }
            }
        }
    }
}

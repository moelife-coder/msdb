use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::secretbox;
use std::collections::HashMap;
use std::path;
mod binary_io;
mod blockencrypt;
mod blocks;
mod db_commands;
mod metadata;
mod utils;
const VERSION_NUMBER: u8 = 4;
fn main() {
    let mut current_location = db_commands::DatabaseLocation::new();
    let mut password: (secretbox::Key, bool) = (secretbox::gen_key(), false);
    let mut main_metadata: metadata::Metadata = metadata::Metadata::create();
    let mut structure_cache: HashMap<
        [u8; blocks::CELL_IDENTIFIER_LENGTH as usize],
        db_commands::Structure,
    > = HashMap::new();
    sodiumoxide::init().expect("Unable to initialize SoldiumMoxide");
    let mut rl = rustyline::Editor::<()>::new();
    loop {
        let user_input = {
            let p = format!("{} >", current_location);
            let readline = rl.readline(&p);
            match readline {
                Ok(line) => line,
                Err(rustyline::error::ReadlineError::Interrupted)
                | Err(rustyline::error::ReadlineError::Eof) => utils::exit(),
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
                    let directory = if let Some(k) = parsed_command.next() {
                        k.to_string()
                    } else {
                        let mut database_directory_ok: bool = false;
                        let mut directory = String::new();
                        while !database_directory_ok {
                            let readline = rl.readline("Database directory:(relative) ");
                            if let Ok(line) = readline {
                                directory = line
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
                    if !path::Path::new(&directory).is_relative() {
                        println!("Please enter a relative path");
                    } else if path::Path::new(&directory).exists() {
                        println!("Directory {} already exists", directory);
                    } else {
                        let password = rpassword::prompt_password_stdout("Password: ")
                            .expect("Unable to read password using rpassword");
                        {
                            let mut password_comfirm = String::new();
                            while password != password_comfirm {
                                password_comfirm = rpassword::prompt_password_stdout(
                                    "Confirm password: ",
                                )
                                .expect("Unable to read password confirmation using rpassword");
                            }
                        }
                        utils::new_database(&directory, &password, VERSION_NUMBER);
                    }
                }
                "decrypt" => {
                    if password.1 {
                        println!("The database has already been unlocked.");
                    } else {
                        let try_database = if let Some(i) = parsed_command.next() {
                            i.to_string()
                        } else {
                            let mut directory = String::new();
                            let readline = rl.readline("Database directory:");
                            if let Ok(line) = readline {
                                directory = line;
                            };
                            directory
                        };
                        let try_passwd = {
                            let password_raw: String;
                            password_raw = match parsed_command.next() {
                                Some(i) => i.to_string(),
                                None => rpassword::prompt_password_stdout("Password: ").unwrap(),
                            };
                            let salt = {
                                let salt_directory = format!("{}/salt", try_database);
                                let salt_vec = binary_io::read_all(&salt_directory);
                                pwhash::Salt::from_slice(&salt_vec[..]).unwrap()
                            };
                            blockencrypt::password_deriv(&password_raw, salt)
                        };
                        main_metadata =
                            utils::select_database(&try_database, &try_passwd, VERSION_NUMBER);
                        current_location.select_root(try_database);
                        password = (try_passwd, true);
                    }
                }
                "logout" => {
                    if password.1 {
                        main_metadata.clear();
                        password = (secretbox::gen_key(), false);
                        current_location.logout();
                    } else {
                        println!("You are not logged in.");
                    }
                }
                "exit" => utils::exit(),
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
                        println!("Unknown command {}", user_input)
                    }
                }
            }
        }
    }
}

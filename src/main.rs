#![feature(test)]
use clap::{App, Arg};
use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::secretbox;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path;
use std::path::Path;
extern crate test;
mod binary_io;
mod blockencrypt;
mod blocks;
mod db_commands;
mod metadata;
mod utils;
const VERSION_NUMBER: u8 = 4;
fn main() {
    let matches = App::new("Mobile Secure DataBase (msdb)")
        .version("Version 0.4 (db version code 4)")
        .author("moelife-coder <61054382+moelife-coder@users.noreply.github.com>")
        .about("A user-friendly, secure and standalone database")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Load MSDB script from a file"),
        )
        .get_matches();

    if matches.is_present("input") {
        from_file(matches.value_of("input").unwrap());
    } else {
        main_cli();
    }
}
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn from_file(filename: &str) {
    //A simple command parser
    if let Ok(lines) = read_lines(filename) {
        let mut current_location = db_commands::DatabaseLocation::new();
        let mut password: (secretbox::Key, bool) = (secretbox::gen_key(), false);
        let mut main_metadata: metadata::Metadata = metadata::Metadata::create();
        let mut structure_cache: HashMap<
            [u8; blocks::CELL_IDENTIFIER_LENGTH as usize],
            db_commands::Structure,
        > = HashMap::new();
        sodiumoxide::init().expect("Unable to initialize SoldiumMoxide");
        for line in lines {
            if let Ok(eachline) = line {
                let mut parsed_commands = eachline.split_whitespace();
                if let Some(i) = parsed_commands.next() {
                    match i {
                        "create" => {
                            let database_name = parsed_commands.next().unwrap();
                            let database_password = if let Some(j) = parsed_commands.next() {
                                j.to_string()
                            } else {
                                rpassword::prompt_password_stdout("Password: ")
                                    .expect("Unable to read password using rpassword")
                            };
                            utils::new_database(&database_name, &database_password, VERSION_NUMBER);
                        }
                        "decrypt" => {
                            let database_name = parsed_commands.next().unwrap();
                            let try_passwd = {
                                let database_password = if let Some(j) = parsed_commands.next() {
                                    j.to_string()
                                } else {
                                    rpassword::prompt_password_stdout("Password: ")
                                        .expect("Unable to read password using rpassword")
                                };
                                let salt = {
                                    let salt_directory = format!("{}/salt", database_name);
                                    let salt_vec = binary_io::read_all(&salt_directory);
                                    pwhash::Salt::from_slice(&salt_vec[..]).unwrap()
                                };
                                blockencrypt::password_deriv(&database_password, salt)
                            };
                            main_metadata =
                                utils::select_database(&database_name, &try_passwd, VERSION_NUMBER);
                            current_location.select_root(database_name.to_string());
                            password = (try_passwd, true);
                        }
                        "exit" => utils::exit(),
                        _ => {
                            if password.1 {
                                db_commands::run_commands(
                                    &eachline,
                                    &mut main_metadata,
                                    &mut current_location,
                                    &password.0,
                                    &mut structure_cache,
                                );
                            } else {
                                panic!("Database unavailable");
                            }
                        }
                    }
                }
            }
        }
    }
}
fn main_cli() {
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
                            &user_input,
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
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[bench]
    fn create_database(b: &mut Bencher) {
        b.iter(|| {
            utils::new_database("testdb", "password", VERSION_NUMBER);
        });
        std::fs::remove_dir_all("testdb").unwrap();
    }
    #[bench]
    fn load_empty_database(b: &mut Bencher) {
        utils::new_database("testdb", "password", VERSION_NUMBER);
        let try_passwd = {
            let password_raw = "password";
            let salt = {
                let salt_vec = binary_io::read_all("testdb/salt");
                pwhash::Salt::from_slice(&salt_vec[..]).unwrap()
            };
            blockencrypt::password_deriv(&password_raw, salt)
        };
        b.iter(|| {
            utils::select_database("testdb", &try_passwd, VERSION_NUMBER);
        });
        std::fs::remove_dir_all("testdb").unwrap();
    }
    #[bench]
    fn create_empty_struct(b: &mut Bencher) {
        utils::new_database("testdb", "password", VERSION_NUMBER);
        let try_passwd = {
            let password_raw = "password";
            let salt = {
                let salt_vec = binary_io::read_all("testdb/salt");
                pwhash::Salt::from_slice(&salt_vec[..]).unwrap()
            };
            blockencrypt::password_deriv(&password_raw, salt)
        };
        let mut main_metadata = utils::select_database("testdb", &try_passwd, VERSION_NUMBER);
        let mut current_location = db_commands::DatabaseLocation::new();
        current_location.select_root("testdb".to_string());
        b.iter(|| {
            db_commands::create_structure(
                "testname",
                &try_passwd,
                &mut main_metadata,
                &mut current_location,
                None,
            )
        });
        std::fs::remove_dir_all("testdb").unwrap();
    }
}

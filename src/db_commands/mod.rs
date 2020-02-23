use super::binary_io;
use super::blockencrypt;
use super::blocks;
use super::metadata;
use rand::{distributions::Uniform, Rng};
use sodiumoxide::crypto::secretbox;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path;
pub const METADATA_INDEX_LEN: u32 = 8;
fn random_metadata_identifier() -> [u8; METADATA_INDEX_LEN as usize] {
    let mut rng = rand::thread_rng();
    let range = Uniform::new(0, 255);
    let vals: Vec<u8> = (0..u8::max_value()).map(|_| rng.sample(&range)).collect();
    vals[0..METADATA_INDEX_LEN as usize].try_into().unwrap()
}
fn random_blocks_identifier() -> [u8; blocks::CELL_IDENTIFIER_LENGTH as usize] {
    let mut rng = rand::thread_rng();
    let range = Uniform::new(0, 255);
    let vals: Vec<u8> = (0..u8::max_value()).map(|_| rng.sample(&range)).collect();
    vals[0..blocks::CELL_IDENTIFIER_LENGTH as usize]
        .try_into()
        .unwrap()
}
pub struct Structure {
    pub metadata: metadata::Metadata,
    pub list: blocks::BlockQueue,
    pub cached_block: HashMap<[u8; blocks::CELL_IDENTIFIER_LENGTH as usize], blocks::BlockQueue>,
}
impl std::fmt::Display for Structure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut cached_block_string = String::new();
        for (i, j) in &self.cached_block {
            cached_block_string.push('[');
            cached_block_string.push_str(&into_hex_block(*i));
            cached_block_string.push(']');
            cached_block_string.push_str(&format!("{}", j));
        }
        write!(
            f,
            "----Metadata----\n{}\n----List----\n{}\n----Cached Block----\n{}\n",
            self.metadata, self.list, cached_block_string
        )
    }
}
pub struct DatabaseLocation {
    root_folder: Option<String>,
    current_structure: Option<([u8; METADATA_INDEX_LEN as usize], String)>,
    current_object: Option<([u8; METADATA_INDEX_LEN as usize], String)>,
    current_cell: Option<([u8; METADATA_INDEX_LEN as usize], String)>,
}
impl DatabaseLocation {
    pub const fn new() -> Self {
        Self {
            current_structure: None,
            current_object: None,
            current_cell: None,
            root_folder: None,
        }
    }
    pub fn logout(&mut self) {
        self.current_structure = None;
        self.current_cell = None;
        self.root_folder = None;
        self.current_object = None;
    }
    pub fn select_structure(&mut self, structure: ([u8; METADATA_INDEX_LEN as usize], String)) {
        if self.root_folder.is_none() {
            panic!("Attempting to select structure while root folder is not selected");
        };
        self.current_structure = Some(structure);
        self.current_cell = None;
    }
    pub fn select_cell(&mut self, cell: ([u8; METADATA_INDEX_LEN as usize], String)) {
        if self.current_object.is_none() {
            panic!("Attempting to select cell while object is not selected");
        };
        self.current_cell = Some(cell);
    }
    pub fn select_object(&mut self, object: ([u8; METADATA_INDEX_LEN as usize], String)) {
        if self.current_structure.is_none() {
            panic!("Attempting to select object while structure is not selected");
        };
        self.current_object = Some(object);
    }
    pub fn select_root(&mut self, root: String) {
        self.root_folder = Some(root);
        self.current_structure = None;
        self.current_cell = None;
    }
    pub fn deselect_cell(&mut self) {
        self.current_cell = None;
    }
    pub fn deselect_object(&mut self) {
        self.current_cell = None;
        self.current_object = None;
    }
    pub fn deselect_structure(&mut self) {
        self.current_structure = None;
        self.current_cell = None;
        self.current_object = None;
    }
    pub fn root_folder(&self) -> Option<&String> {
        self.root_folder.as_ref()
    }
    pub fn current_structure_identifier(&self) -> Option<[u8; METADATA_INDEX_LEN as usize]> {
        match self.current_structure {
            None => None,
            Some((i, _)) => Some(i),
        }
    }
    pub fn current_structure_pretty_name(&self) -> Option<&str> {
        match self.current_structure.as_ref() {
            None => None,
            Some((_, i)) => Some(i),
        }
    }
    pub fn current_cell_identifier(&self) -> Option<[u8; METADATA_INDEX_LEN as usize]> {
        match self.current_cell {
            None => None,
            Some((i, _)) => Some(i),
        }
    }
    pub fn current_cell_pretty_name(&self) -> Option<&str> {
        match self.current_cell.as_ref() {
            None => None,
            Some((_, i)) => Some(i),
        }
    }
    pub fn current_object_identifier(&self) -> Option<[u8; METADATA_INDEX_LEN as usize]> {
        match self.current_object {
            None => None,
            Some((i, _)) => Some(i),
        }
    }
    pub fn current_object_pretty_name(&self) -> Option<&str> {
        match self.current_object.as_ref() {
            None => None,
            Some((_, i)) => Some(i),
        }
    }
}
impl std::fmt::Display for DatabaseLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.root_folder == None {
            write!(f, "")
        } else if self.current_structure == None {
            write!(f, "~",)
        } else if self.current_object == None {
            write!(f, "{}", self.current_structure_pretty_name().unwrap())
        } else if self.current_cell == None {
            write!(
                f,
                "{}/{}",
                self.current_structure_pretty_name().unwrap(),
                self.current_object_pretty_name().unwrap()
            )
        } else {
            write!(
                f,
                "{}/{}: {}",
                self.current_structure_pretty_name().unwrap(),
                self.current_object_pretty_name().unwrap(),
                self.current_cell_pretty_name().unwrap(),
            )
        }
    }
}
fn into_hex_metadata(identifier: [u8; METADATA_INDEX_LEN as usize]) -> String {
    hex::encode(identifier.to_vec())
}
fn into_hex_block(identifier: [u8; blocks::CELL_IDENTIFIER_LENGTH as usize]) -> String {
    hex::encode(identifier.to_vec())
}
fn from_hex_metadata(encoded_string: &str) -> [u8; METADATA_INDEX_LEN as usize] {
    let result = hex::decode(encoded_string).expect("Error when decoding identifier");
    result[0..METADATA_INDEX_LEN as usize].try_into().unwrap()
}
fn from_hex_blocks(encoded_string: &str) -> [u8; blocks::CELL_IDENTIFIER_LENGTH as usize] {
    let result = hex::decode(encoded_string).expect("Error when decoding identifier");
    result[0..blocks::CELL_IDENTIFIER_LENGTH as usize]
        .try_into()
        .unwrap()
}
/// Execute database commands
pub fn run_commands(
    argument: &str,
    main_metadata: &mut metadata::Metadata,
    current_location: &mut DatabaseLocation,
    password: &secretbox::Key,
    structure_cache: &mut HashMap<[u8; blocks::CELL_IDENTIFIER_LENGTH as usize], Structure>,
) {
    let mut parsed_command = argument.split_whitespace();
    match parsed_command.next().unwrap() {
        "new" => {
            match parsed_command.next() {
                None => {
                    println!("You need to have at least one argument for 'new' command");
                }
                Some(i) => match i {
                    "struct" => match parsed_command.next() {
                        None => println!("You need to specify structure name"),
                        Some(j) => {
                            if main_metadata.sub_data().get(&j.to_string()).is_some() {
                                println!("Structure {} already exists", j);
                            } else {
                                create_structure(
                                    j,
                                    password,
                                    main_metadata,
                                    current_location,
                                    None,
                                );
                            }
                        }
                    },
                    j => match current_location.current_structure_identifier() {
                        None => println!("You cannot create a object under root.\nPlease select/create a structure before creating object."),
                        Some(_) => {
                            if current_location.current_object_identifier().is_none() {
                                let exist = {
                                    //Check if the object exist
                                    let cells = &structure_cache
                                        .get(
                                            &current_location
                                                .current_structure_identifier()
                                                .expect(
                                                    "Unable to read current structure identifier",
                                                ),
                                        )
                                        .expect("Unable to read current structure from cache")
                                        .list
                                        .cells;
                                    let mut object_exist = false;
                                    //TODO: Optimize this code
                                    for k in cells {
                                        if let blocks::Cell::Literal(l, _) = k {
                                            if !object_exist && j == l {
                                                object_exist = true;
                                            }
                                        }
                                    }
                                    object_exist
                                };
                                if exist {
                                    println!("Object already exists. Please try another name.");
                                } else {
                                    //Create a new object
                                    create_object(j, current_location, structure_cache);
                                };
                            } else if j == "list" {
                                println!("You cannot create a cell with reserved name `list`. Please try another name.");
                            } else {
                                //Create a new cell
                                let cell_type = {
                                    if let Some(k) = parsed_command.next() {
                                        let k = k.to_ascii_lowercase();
                                        match k.as_str() {
                                            //TODO: Support Incomplete Block
                                            "literal" | "blob" | "link" | "revlink" => k,
                                            _ => {
                                                println!("Cell type not reconized. Treated as Literal Cell.");
                                                "literal".to_string()
                                            }
                                        }
                                    } else {
                                        println!("Creating a cell with no type is discouraged. Treated as Literal Cell.");
                                        "literal".to_string()
                                    }
                                };
                                let cell_content = {
                                    if let Some(k) = parsed_command.next() {
                                        k
                                    } else {
                                        println!("Creating a cell with no content is discouraged.");
                                        "0"
                                    }
                                };
                                create_cell(
                                    j,
                                    &cell_type,
                                    cell_content,
                                    password,
                                    current_location,
                                    structure_cache,
                                );
                            }
                        }
                    },
                },
            };
        }
        "alter" => {
            if current_location.current_object_identifier() == None {
                println!("You cannot alter a cell outside objects");
            } else {
                match parsed_command.next() {
                    None => println!("alter command requires three arguments"),
                    Some(i) => match parsed_command.next() {
                        None => println!("alter command requires three arguments"),
                        Some(j) => match parsed_command.next() {
                            None => println!("alter command requires three arguments"),
                            Some(k) => {
                                alter_cell(i, j, k, password, current_location, structure_cache)
                            }
                        },
                    },
                }
            }
        }
        "ls" => {
            if current_location.current_structure_identifier() == None {
                ugly_print_structure(main_metadata);
            } else if current_location.current_object_identifier() == None {
                ugly_print_objects(current_location, structure_cache);
            } else {
                ugly_print_cell(current_location, structure_cache, main_metadata);
            }
        }
        "debls" => {
            if current_location.current_structure_identifier() == None {
                debug_print_structure(main_metadata);
            } else if current_location.current_object_identifier() == None {
                debug_print_objects(current_location, structure_cache);
            } else {
                debug_print_cell(current_location, structure_cache);
            }
        }
        "leave" => leave(current_location),
        "select" => match parsed_command.next() {
            None => println!("select command requires exactly one command"),
            Some(i) => {
                if current_location.current_structure_identifier() == None {
                    select_structure(
                        i,
                        password,
                        main_metadata,
                        current_location,
                        structure_cache,
                    );
                } else if current_location.current_object_identifier() == None {
                    select_object(i, current_location, structure_cache);
                } else if current_location.current_cell_identifier() == None {
                    let field_identifier = from_hex_blocks(
                        structure_cache
                            .get(&current_location.current_structure_identifier().unwrap())
                            .unwrap()
                            .metadata
                            .sub_data()
                            .get(&i.to_string())
                            .expect("Unable to find field"),
                    );
                    current_location.select_cell((field_identifier, i.to_string()));
                } else {
                    println!("Please return to root before select");
                }
            }
        },
        "clean" => {
            clear_cache(structure_cache);
        }
        "sync" => {
            if main_metadata.has_modified() {
                println!("Writing main metadata to disk...");
                {
                    let main_metadata_vec = main_metadata.to_vec();
                    let data = blockencrypt::encrypt_block(&main_metadata_vec, password);
                    let filename = format!("{}/metadata", current_location.root_folder().unwrap());
                    binary_io::write_with_nonce(&filename, &data.0, data.1);
                }
                main_metadata.set_not_modified();
            } else {
                println!("Main metadata not modified; ignoring.");
            }
            for i in structure_cache {
                println!("Writing {} structure to disk...", into_hex_metadata(*i.0));
                {
                    println!("|-Cell list");
                    //TODO: custom cell size
                    i.1.list.cell_to_raw(None, 512);
                    for j in &i.1.list.queue {
                        let data = blockencrypt::encrypt_block(j, password);
                        let filename = format!(
                            "{}/{}/{}",
                            current_location.root_folder().unwrap(),
                            into_hex_metadata(*i.0),
                            i.1.metadata.sub_data().get(&String::from("list")).unwrap()
                        );
                        binary_io::write_with_nonce(&filename, &data.0, data.1);
                    }
                }
                {
                    println!("|-Field Cache");
                    //TODO: custom cell size
                    for j in &mut i.1.cached_block {
                        let folder_name = format!(
                            "{}/{}/{}",
                            current_location.root_folder().unwrap(),
                            into_hex_metadata(*i.0),
                            into_hex_block(*j.0),
                        );
                        if !path::Path::new(&folder_name).is_dir() {
                            fs::create_dir(&folder_name).expect("Unable to create cell folder");
                        }
                        j.1.cell_to_raw(Some(65536), 512);
                        for (current_num, k) in j.1.queue.iter().enumerate() {
                            let data = blockencrypt::encrypt_block(k, password);
                            let filename = format!("{}/{}.blk", folder_name, current_num);
                            binary_io::write_with_nonce(&filename, &data.0, data.1);
                        }
                    }
                }
                {
                    if i.1.metadata.has_modified() {
                        println!("|-Structure Metadata");
                        //Save metadata
                        let metadata_sync_vec = i.1.metadata.to_vec();
                        let data = blockencrypt::encrypt_block(&metadata_sync_vec, password);
                        let filename = format!(
                            "{}/{}/metadata",
                            current_location.root_folder().unwrap(),
                            into_hex_metadata(*i.0)
                        );
                        binary_io::write_with_nonce(&filename, &data.0, data.1);
                        i.1.metadata.set_not_modified();
                    } else {
                        println!("|-Metadata(Ignored)")
                    }
                }
            }
        }
        "load" => {
            //Load a set of cells into cache
            if current_location.current_structure_identifier() == None {
                println!("Please select a structure before loading any cell");
            } else if current_location.current_object_identifier() == None {
                match parsed_command.next() {
                    None => {
                        println!("Loading every field inside structure...");
                        let mut result: Vec<(
                            [u8; blocks::CELL_IDENTIFIER_LENGTH as usize],
                            blocks::BlockQueue,
                        )> = Vec::new();
                        for (i, j) in structure_cache
                            .get(&current_location.current_structure_identifier().unwrap())
                            .unwrap()
                            .metadata
                            .sub_data()
                        {
                            if i.as_str() != "list" {
                                print!(" Loading {} ({})", i, j);
                                let structure_directory = format!(
                                    "{}/{}/{}",
                                    current_location.root_folder().unwrap(),
                                    into_hex_metadata(
                                        current_location.current_structure_identifier().unwrap()
                                    ),
                                    j
                                );
                                let mut temp_block = blocks::BlockQueue::new();
                                //TODO: 加入缓存部分Cell的功能
                                let mut current_num = 0;
                                while path::Path::new(&format!(
                                    "{}/{}.blk",
                                    structure_directory, current_num
                                ))
                                .is_file()
                                {
                                    let block = binary_io::read_with_nonce(&format!(
                                        "{}/{}.blk",
                                        structure_directory, current_num
                                    ));
                                    temp_block.import_from_vec(blockencrypt::decrypt_block(
                                        &block.0, password, block.1,
                                    ));
                                    temp_block.raw_to_cell(512);
                                    print!(".");
                                    current_num += 1;
                                }
                                println!();
                                result.push((from_hex_metadata(j), temp_block));
                            }
                        }
                        for i in result {
                            structure_cache
                                .get_mut(&current_location.current_structure_identifier().unwrap())
                                .unwrap()
                                .cached_block
                                .insert(i.0, i.1);
                        }
                    }
                    Some(k) => {
                        let mut result: Vec<(
                            [u8; blocks::CELL_IDENTIFIER_LENGTH as usize],
                            blocks::BlockQueue,
                        )> = Vec::new();
                        for (i, j) in structure_cache
                            .get(&current_location.current_structure_identifier().unwrap())
                            .unwrap()
                            .metadata
                            .sub_data()
                        {
                            if i.as_str() == k {
                                print!(" Loading {} ({})", i, j);
                                let structure_directory = format!(
                                    "{}/{}/{}",
                                    current_location.root_folder().unwrap(),
                                    into_hex_metadata(
                                        current_location.current_structure_identifier().unwrap()
                                    ),
                                    j
                                );
                                let mut temp_block = blocks::BlockQueue::new();
                                //TODO: 加入缓存部分Cell的功能
                                let mut current_num = 0;
                                while path::Path::new(&format!(
                                    "{}/{}.blk",
                                    structure_directory, current_num
                                ))
                                .is_file()
                                {
                                    let block = binary_io::read_with_nonce(&format!(
                                        "{}/{}.blk",
                                        structure_directory, current_num
                                    ));
                                    temp_block.import_from_vec(blockencrypt::decrypt_block(
                                        &block.0, password, block.1,
                                    ));
                                    print!(".");
                                    current_num += 1;
                                }
                                println!();
                                result.push((from_hex_metadata(j), temp_block));
                            }
                        }
                        for i in result {
                            structure_cache
                                .get_mut(&current_location.current_structure_identifier().unwrap())
                                .unwrap()
                                .cached_block
                                .insert(i.0, i.1);
                        }
                    }
                }
            } else {
                print!(
                    " Loading {} ({})",
                    current_location.current_cell_pretty_name().unwrap(),
                    into_hex_metadata(current_location.current_cell_identifier().unwrap())
                );
                let structure_directory = format!(
                    "{}/{}/{}",
                    current_location.root_folder().unwrap(),
                    into_hex_metadata(current_location.current_structure_identifier().unwrap()),
                    into_hex_metadata(current_location.current_cell_identifier().unwrap())
                );
                let mut temp_block = blocks::BlockQueue::new();
                //TODO: 加入缓存部分Cell的功能
                let mut current_num = 0;
                while path::Path::new(&format!("{}/{}.blk", structure_directory, current_num))
                    .is_file()
                {
                    let block = binary_io::read_with_nonce(&format!(
                        "{}/{}.blk",
                        structure_directory, current_num
                    ));
                    temp_block
                        .import_from_vec(blockencrypt::decrypt_block(&block.0, password, block.1));
                    print!(".");
                    current_num += 1;
                }
                structure_cache
                    .get_mut(&current_location.current_structure_identifier().unwrap())
                    .unwrap()
                    .cached_block
                    .insert(
                        current_location.current_cell_identifier().unwrap(),
                        temp_block,
                    );
            }
        }
        "pwd" => println!("{}", current_location),
        "del" => {
            if let Some(i) = parsed_command.next() {
                if current_location.current_structure_identifier() == None {
                    delete_structure(i, current_location, main_metadata);
                } else if current_location.current_object_identifier() == None {
                    delete_object(i, current_location, structure_cache);
                } else if current_location.current_cell_identifier() == None {
                    delete_cell(i, current_location, structure_cache);
                } else {
                    println!("Please `leave` the cell before deleting it");
                }
            } else {
                println!("`del` command requires exactly one parameter");
            }
        }
        "unload" => {
            if let Some(i) = parsed_command.next() {
                structure_cache
                    .remove(&from_hex_metadata(main_metadata.sub_data().get(i).unwrap()));
            } else {
                println!("`unload` command requires exactly one parameter");
            }
        }
        "setprop" => {
            if let Some(name) = parsed_command.next() {
                if let Some(value) = parsed_command.next() {
                    if current_location.current_structure_identifier() == None {
                        main_metadata.new_attribute(name, value);
                    } else if current_location.current_object_identifier() == None {
                        structure_cache
                            .get_mut(&current_location.current_structure_identifier().unwrap())
                            .unwrap()
                            .metadata
                            .new_attribute(name, value);
                    } else {
                        println!("currently `setprop` only works with main metadata and structure metadata");
                    }
                } else {
                    println!("`setprop` command requires exactly two parameter");
                }
            } else {
                println!("`setprop` command requires exactly two parameter");
            }
        }
        "getprop" => {
            if current_location.current_structure_identifier() == None {
                for (i, j) in main_metadata.attribute() {
                    println!("{}={}", i, j);
                }
            } else if current_location.current_object_identifier() == None {
                for (i, j) in structure_cache
                    .get_mut(&current_location.current_structure_identifier().unwrap())
                    .unwrap()
                    .metadata
                    .attribute()
                {
                    println!("{}={}", i, j);
                }
            } else {
                println!(
                    "currently `getprop` only works with main metadata and structure metadata"
                );
            }
        }
        "show" => {
            if current_location.current_structure_identifier() == None {
                println!("{}", main_metadata);
            } else if current_location.current_object_identifier() == None {
                println!(
                    "{}",
                    structure_cache
                        .get(&current_location.current_structure_identifier().unwrap())
                        .unwrap()
                );
            } else if current_location.current_cell_identifier() == None {
                for i in structure_cache
                    .get(&current_location.current_structure_identifier().unwrap())
                    .unwrap()
                    .cached_block
                    .values()
                {
                    for j in &i.cells {
                        if &current_location.current_object_identifier().unwrap()
                            == match j {
                                blocks::Cell::Literal(_, k)
                                | blocks::Cell::Blob(_, k)
                                | blocks::Cell::Link(_, _, k) => k,
                                blocks::Cell::BlobIncomplete(_, k)
                                | blocks::Cell::LiteralIncomplete(_, k) => &k.identifier,
                            }
                        {
                            println!("{}\n", j);
                        }
                    }
                }
            } else {
                println!("PLEASEIMPLEMENT");
            }
        }
        i => panic!("Unknown command {}", i),
    }
}
/// Create a structure in root.
/// it will *panic* if:
/// 1. Current location does not have a root folder
/// 2. Unable to create structure directory (eg. due to insufficient permission)
pub fn create_structure(
    structure_name: &str,
    password: &secretbox::Key,
    main_metadata: &mut metadata::Metadata,
    current_location: &mut DatabaseLocation,
    default_cell_list_size: Option<u32>,
) {
    let default_cell_list_size = if let Some(i) = default_cell_list_size {
        i
    } else {
        32
    };
    //Create a structure token
    let structure_token = {
        let mut tk = random_metadata_identifier();
        let mut structure_directory = format!(
            "{}/{}",
            current_location.root_folder().unwrap(),
            into_hex_metadata(tk)
        );
        while path::Path::new(&structure_directory).exists() {
            tk = random_metadata_identifier();
            structure_directory = format!(
                "{}/{}",
                current_location.root_folder().unwrap(),
                into_hex_metadata(tk)
            );
        }
        tk
    };
    //Create structure directory
    std::fs::create_dir(format!(
        "{}/{}",
        current_location.root_folder().unwrap(),
        into_hex_metadata(structure_token)
    ))
    .expect("Unable to create structure directory");
    //Insert structure directory into main metadata
    main_metadata.new_sub_data(structure_name, &into_hex_metadata(structure_token));
    //Write structure metadata
    {
        let mut structure_data = metadata::Metadata::create();
        structure_data.new_attribute(&String::from("type"), &String::from("struct"));
        structure_data.new_attribute(
            &String::from("size"),
            &format!("{}", default_cell_list_size),
        );
        //Create a identifier for list
        let list_identifier = random_metadata_identifier();
        structure_data.new_sub_data(&String::from("list"), &into_hex_metadata(list_identifier));
        //Write metadata into file
        let metadata_vec = structure_data.into_vec();
        let data = blockencrypt::encrypt_block(&metadata_vec, password);
        let filename = format!(
            "{}/{}/metadata",
            current_location.root_folder().unwrap(),
            into_hex_metadata(structure_token)
        );
        binary_io::write_with_nonce(&filename, &data.0, data.1);
    }
    println!(
        "Structure {}[identifier {}] created.",
        structure_name,
        into_hex_metadata(structure_token)
    );
}
/// Create a object in current structure
fn create_object(
    object_name: &str,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    //Create a object identifier
    let object_identifier = {
        let mut tk = random_blocks_identifier();
        while {
            let mut result = false;
            for k in &structure_cache
                .get(&current_location.current_structure_identifier().unwrap())
                .unwrap()
                .list
                .cells
            {
                if !result {
                    if let blocks::Cell::Literal(_, l) = k {
                        if *l == tk {
                            result = true;
                        }
                    }
                }
            }
            result
        } {
            tk = random_blocks_identifier();
        }
        tk
    };
    //Insert into structure cache
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .list
        .import_cell(blocks::Cell::Literal(
            object_name.to_string(),
            object_identifier,
        ));
    println!(
        "Object {}[identifier: {}] created.",
        object_name,
        into_hex_block(object_identifier)
    );
}
/// Create a field in current structure
fn create_field(
    field_name: &str,
    password: &secretbox::Key,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
    default_cell_size: Option<u32>,
) -> [u8; METADATA_INDEX_LEN as usize] {
    let default_cell_size = if let Some(k) = default_cell_size {
        k
    } else {
        32
    };
    //Create a field identifier
    let field_identifier = {
        let mut identifier = random_metadata_identifier();
        while {
            structure_cache
                .get(&current_location.current_structure_identifier().unwrap())
                .expect("Unable to read structure cache metadata")
                .metadata
                .sub_data()
                .get(field_name)
                .is_some()
        } {
            identifier = random_metadata_identifier();
        }
        identifier
    };
    //Write identifier into metadata
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .metadata
        .new_sub_data(
            &field_name.to_string(),
            &into_hex_metadata(field_identifier),
        );
    //Create a empty structure cache in structure cache
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
        .insert(field_identifier, blocks::BlockQueue::new());
    //Create field
    fs::create_dir(format!(
        "{}/{}/{}",
        current_location.root_folder().unwrap(),
        into_hex_metadata(current_location.current_structure_identifier().unwrap()),
        into_hex_metadata(field_identifier)
    ))
    .expect("Unable to create directory");
    //Create a metadata, containing the default block size for current field
    {
        let mut field_metadata = metadata::Metadata::create();
        field_metadata.new_attribute("size", &format!("{}", default_cell_size));
        let field_metadata_vec = field_metadata.into_vec();
        let data = blockencrypt::encrypt_block(&field_metadata_vec, password);
        let filename = format!(
            "{}/{}/{}/metadata",
            current_location.root_folder().unwrap(),
            into_hex_metadata(current_location.current_structure_identifier().unwrap()),
            into_hex_metadata(field_identifier)
        );
        binary_io::write_with_nonce(&filename, &data.0, data.1);
    }
    println!(
        "Field {}[identifier: {}] created.",
        field_name,
        into_hex_metadata(field_identifier)
    );
    field_identifier
}
/// Change current structure to desired structure
/// Also load structure metadata and cell list into cache
fn select_structure(
    structure_name: &str,
    password: &secretbox::Key,
    main_metadata: &mut metadata::Metadata,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    let structure_token_without_unwarp = main_metadata.sub_data().get(structure_name);
    if let Some(structure_token) = structure_token_without_unwarp {
        let structure_metadata = {
            let structure_list_path = format!(
                "{}/{}/metadata",
                current_location.root_folder().unwrap(),
                structure_token
            );
            let structure_list_raw = binary_io::read_with_nonce(&structure_list_path);
            let structure_list_vec =
                blockencrypt::decrypt_block(&structure_list_raw.0, password, structure_list_raw.1);
            metadata::Metadata::from_vec(structure_list_vec)
        };
        let cell_list = {
            let block_list_path = format!(
                "{}/{}/{}",
                current_location.root_folder().unwrap(),
                structure_token,
                structure_metadata
                    .sub_data()
                    .get(&String::from("list"))
                    .unwrap()
            );
            if path::Path::new(&block_list_path).is_file() {
                //Read cell list
                let block_list_raw = binary_io::read_with_nonce(&block_list_path);
                let block_list_vec =
                    blockencrypt::decrypt_block(&block_list_raw.0, password, block_list_raw.1);
                blocks::BlockQueue::from_vec(
                    block_list_vec,
                    structure_metadata
                        .attribute()
                        .get(&String::from("size"))
                        .unwrap()
                        .parse()
                        .unwrap(),
                )
            } else {
                //Create a new cell list
                blocks::BlockQueue::new()
            }
        };
        structure_cache.insert(
            from_hex_metadata(structure_token),
            Structure {
                metadata: structure_metadata,
                list: cell_list,
                cached_block: HashMap::new(),
            },
        );
        current_location.select_structure((
            from_hex_metadata(structure_token),
            structure_name.to_string(),
        ));
        println!("Structure {}[{}]", structure_name, structure_token);
    } else {
        println!(
            "Unable to select structure {}: No such structure",
            structure_name
        );
    }
}
/// Change current object to desired object
fn select_object(
    object_name: &str,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    let object_identifier = {
        let mut id: Option<[u8; blocks::CELL_IDENTIFIER_LENGTH as usize]> = None;
        let object_list = &structure_cache
            .get(&current_location.current_structure_identifier().unwrap())
            .unwrap()
            .list;
        for individual in &object_list.cells {
            if let blocks::Cell::Literal(x, y) = individual {
                if x == object_name {
                    id = Some(*y);
                }
            }
        }
        id
    };
    if let Some(id) = object_identifier {
        current_location.select_object((id, object_name.to_string()));
    } else {
        println!("Unable to select structure {}: No such object", object_name);
    }
}
/// Remove all cached block inside structures
///
/// **Highly Unrecomended** because it will also remove `list` from `cached_block`, breaking things.
fn clear_cache(structure_cache: &mut HashMap<[u8; 8], Structure>) {
    for i in structure_cache {
        println!("Structure {}", into_hex_metadata(*i.0));
        for j in &mut i.1.cached_block {
            j.1.clean_cells();
        }
    }
}
fn alter_cell(
    cell_name: &str,
    cell_type: &str,
    cell_content: &str,
    password: &secretbox::Key,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    //First, delete the cell
    delete_cell(cell_name, current_location, structure_cache);
    //Then, create a new cell
    create_cell(
        cell_name,
        cell_type,
        cell_content,
        password,
        current_location,
        structure_cache,
    );
}
/// Delete cell
fn delete_cell(
    cell_name: &str,
    current_location: &DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    let field = structure_cache
        .get_mut(
            &current_location
                .current_structure_identifier()
                .expect("Unable to find current structure identifier"),
        )
        .expect("Unable to read structure cache metadata");
    let field_identifier = field.metadata.sub_data().get(cell_name);
    if let Some(identifier) = field_identifier {
        field
            .cached_block
            .get_mut(&from_hex_metadata(identifier))
            .unwrap()
            .delete_cell(current_location.current_object_identifier().unwrap());
    } else {
        println!("Cannot delete cell {}: cell field not exist", cell_name);
    }
}
/// Delete object
fn delete_object(
    object_name: &str,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    let field = structure_cache
        .get_mut(
            &current_location
                .current_structure_identifier()
                .expect("Unable to find current structure identifier"),
        )
        .expect("Unable to read structure cache metadata");
    field
        .cached_block
        .get_mut(&from_hex_metadata(
            field.metadata.sub_data().get("list").unwrap(),
        ))
        .unwrap()
        .delete_literal_cell_based_on_content(object_name);
}
fn delete_structure(
    structure_name: &str,
    current_location: &mut DatabaseLocation,
    main_metadata: &mut metadata::Metadata,
) {
    if let Some(identifier) = main_metadata.sub_data().get(structure_name) {
        if std::fs::remove_dir(format!(
            "{}/{}",
            current_location.root_folder().unwrap(),
            identifier
        ))
        .is_err()
        {
            println!("Error happends when removing structure")
        } else {
            main_metadata.delete_sub_data(structure_name);
        }
    } else {
        println!(
            "Unable to remove structure: structure {} does not exist",
            structure_name
        );
    }
}
/// Create a cell in current object
fn create_cell(
    field_name: &str,
    cell_type: &str,
    cell_content: &str,
    password: &secretbox::Key,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    //Create a new cell
    let insert_cell = {
        match cell_type {
            "literal" => blocks::Cell::Literal(
                cell_content.to_string(),
                current_location.current_object_identifier().unwrap(),
            ),
            "blob" => blocks::Cell::Blob(
                binary_io::read_all(cell_content),
                current_location.current_object_identifier().unwrap(),
            ),
            "link" | "revlink" => {
                let link_target: Vec<&str> = cell_content.split('/').collect();
                blocks::Cell::Link(
                    if "link" == cell_type {
                        blocks::LinkType::Forward
                    } else {
                        blocks::LinkType::Reverse
                    },
                    match link_target.len() {
                        1 => blocks::LinkTarget::SameBlock(from_hex_blocks(
                            &link_target[0].to_string(),
                        )),
                        2 => blocks::LinkTarget::AnotherField(
                            from_hex_metadata(&link_target[0].to_string()),
                            from_hex_blocks(&link_target[1].to_string()),
                        ),
                        3 => blocks::LinkTarget::AnotherStruct(
                            from_hex_metadata(&link_target[0].to_string()),
                            from_hex_metadata(&link_target[1].to_string()),
                            from_hex_blocks(&link_target[2].to_string()),
                        ),
                        _ => panic!("Unexpected link format"),
                    },
                    current_location.current_object_identifier().unwrap(),
                )
            }
            _ => {
                println!("Warning: cell type not reconized. Treated as Literal Cell.");
                blocks::Cell::Literal(
                    cell_content.to_string(),
                    current_location.current_object_identifier().unwrap(),
                )
            }
        }
    };
    //Get field identifier
    let field_identifier = if let Some(k) = structure_cache
        .get(
            &current_location
                .current_structure_identifier()
                .expect("Unable to find current structure identifier"),
        )
        .expect("Unable to read structure cache metadata")
        .metadata
        .sub_data()
        .get(field_name)
    {
        let result = k;
        result.to_string()
    } else {
        //Create a new field
        into_hex_metadata(create_field(
            field_name,
            password,
            current_location,
            structure_cache,
            Some(cell_content.len().try_into().unwrap()),
        ))
    };
    //Insert cell into structure cache
    if structure_cache
        .get(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
        .get(&from_hex_metadata(&field_identifier))
        .is_none()
    {
        println!("Please cache the cell before writing");
    } else {
        structure_cache
            .get_mut(&current_location.current_structure_identifier().unwrap())
            .unwrap()
            .cached_block
            .get_mut(&from_hex_metadata(&field_identifier))
            .unwrap()
            .import_cell(insert_cell);
    }
}

/// Exit current structure/object/cell
fn leave(current_location: &mut DatabaseLocation) {
    if current_location.current_structure_identifier() != None {
        if current_location.current_object_identifier() == None {
            current_location.deselect_structure();
        } else if current_location.current_cell_identifier() == None {
            current_location.deselect_object();
        } else {
            current_location.deselect_cell();
        }
    }
}
/// Debug version of printing structures inside main metadata
fn debug_print_structure(main_metadata: &metadata::Metadata) {
    println!("[Config]");
    for (i, j) in main_metadata.attribute() {
        println!("{} = {}", i, j);
    }
    println!("[Structure]");
    for (i, j) in main_metadata.sub_data() {
        println!("{} -> {}", i, j);
    }
}
/// Ugly version of printing structures inside main metadata
fn ugly_print_structure(main_metadata: &metadata::Metadata) {
    println!("[Structure]");
    for i in main_metadata.sub_data().keys() {
        println!("{}", i);
    }
}
///Debug version of printing objects inside structure
fn debug_print_objects(
    current_location: &DatabaseLocation,
    structure_cache: &HashMap<[u8; 8], Structure>,
) {
    for i in &structure_cache
        .get(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .list
        .cells
    {
        println!(
            "{} -> {}",
            if let blocks::Cell::Literal(j, _) = i {
                j
            } else {
                "null"
            },
            if let blocks::Cell::Literal(_, j) = i {
                into_hex_block(*j)
            } else {
                "null".to_string()
            }
        );
    }
}
///Ugly version of printing objects inside structure
fn ugly_print_objects(
    current_location: &DatabaseLocation,
    structure_cache: &HashMap<[u8; 8], Structure>,
) {
    for i in &structure_cache
        .get(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .list
        .cells
    {
        println!(
            "{}",
            if let blocks::Cell::Literal(j, _) = i {
                j
            } else {
                "null"
            }
        );
    }
}
/// Ugly version of printing cells inside current object
fn ugly_print_cell(
    current_location: &DatabaseLocation,
    structure_cache: &HashMap<[u8; 8], Structure>,
    main_metadata: &metadata::Metadata,
) {
    let current_object = current_location.current_object_identifier().unwrap();
    for (i, j) in &structure_cache
        .get(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
    {
        let current_field = {
            let mut result: &str = "";
            for k in structure_cache
                .get(&current_location.current_structure_identifier().unwrap())
                .unwrap()
                .metadata
                .sub_data()
            {
                if k.1 == &into_hex_metadata(*i) {
                    result = k.0;
                }
            }
            result
        };
        for k in &j.cells {
            //Get current field name and identifier
            if match k {
                blocks::Cell::Literal(_, l)
                | blocks::Cell::Blob(_, l)
                | blocks::Cell::Link(_, _, l) => *l,
                blocks::Cell::LiteralIncomplete(_, l) | blocks::Cell::BlobIncomplete(_, l) => {
                    l.identifier
                }
            } == current_object
            {
                println!(
                    "{}",
                    match k {
                        blocks::Cell::Literal(m, _) => format!("{} : \"{}\"", current_field, m),
                        blocks::Cell::Blob(m, _) =>
                            format!("{}: {}", current_field, hex::encode(m)),
                        blocks::Cell::Link(m, n, _) => {
                            format!(
                                "{}: {} Link to {}",
                                current_field,
                                match m {
                                    blocks::LinkType::Forward => "Forward",
                                    blocks::LinkType::Reverse => "Reverse",
                                },
                                match n {
                                    blocks::LinkTarget::SameBlock(o) => {
                                        let mut result_cell_name = String::new();
                                        for p in &structure_cache
                                            .get(
                                                &current_location
                                                    .current_structure_identifier()
                                                    .unwrap(),
                                            )
                                            .unwrap()
                                            .list
                                            .cells
                                        {
                                            if let blocks::Cell::Literal(r, q) = p {
                                                if q == o {
                                                    result_cell_name = r.to_string();
                                                }
                                            }
                                        }
                                        format!("{}'s same cell", result_cell_name)
                                    }
                                    blocks::LinkTarget::AnotherField(o, p) => {
                                        let mut result_cell_name = String::new();
                                        for q in &structure_cache
                                            .get(
                                                &current_location
                                                    .current_structure_identifier()
                                                    .unwrap(),
                                            )
                                            .unwrap()
                                            .list
                                            .cells
                                        {
                                            if let blocks::Cell::Literal(s, r) = q {
                                                if r == p {
                                                    result_cell_name = s.to_string();
                                                }
                                            }
                                        }
                                        let result_field_name = &structure_cache
                                            .get(
                                                &current_location
                                                    .current_structure_identifier()
                                                    .unwrap(),
                                            )
                                            .unwrap()
                                            .metadata
                                            .sub_data()
                                            .get(&into_hex_metadata(*o))
                                            .unwrap();
                                        format!("{}'s {}", result_cell_name, result_field_name)
                                    }
                                    blocks::LinkTarget::AnotherStruct(o, p, q) => {
                                        let mut result_cell_name = String::new();
                                        for r in &structure_cache
                                            .get(
                                                &current_location
                                                    .current_structure_identifier()
                                                    .unwrap(),
                                            )
                                            .unwrap()
                                            .list
                                            .cells
                                        {
                                            if let blocks::Cell::Literal(t, s) = r {
                                                if s == q {
                                                    result_cell_name = t.to_string();
                                                }
                                            }
                                        }
                                        let result_field_name = &structure_cache
                                            .get(
                                                &current_location
                                                    .current_structure_identifier()
                                                    .unwrap(),
                                            )
                                            .unwrap()
                                            .metadata
                                            .sub_data()
                                            .get(&into_hex_metadata(*p))
                                            .unwrap();
                                        let result_struct_name = &main_metadata
                                            .sub_data()
                                            .get(&into_hex_metadata(*o))
                                            .unwrap();
                                        format!(
                                            "{}/{}'s {}",
                                            result_struct_name, result_cell_name, result_field_name
                                        )
                                    }
                                }
                            )
                        }
                        blocks::Cell::BlobIncomplete(m, _) =>
                            format!("{}: [BlobIncomplete] {}", current_field, hex::encode(m)),
                        blocks::Cell::LiteralIncomplete(m, _) =>
                            format!("{}: [LiteralIncomplete] {}", current_field, hex::encode(m)),
                    }
                )
            }
        }
    }
}
/// Debug version of printing cells inside current object
fn debug_print_cell(
    current_location: &DatabaseLocation,
    structure_cache: &HashMap<[u8; 8], Structure>,
) {
    let current_object = current_location.current_object_identifier().unwrap();
    for (i, j) in &structure_cache
        .get(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
    {
        let current_field = format!(
            "{}[{}]",
            {
                let mut result: &str = "";
                for k in structure_cache
                    .get(&current_location.current_structure_identifier().unwrap())
                    .unwrap()
                    .metadata
                    .sub_data()
                {
                    if k.1 == &into_hex_metadata(*i) {
                        result = k.0;
                    }
                }
                result
            },
            into_hex_metadata(*i)
        );
        for k in &j.cells {
            //Get current field name and identifier
            if match k {
                blocks::Cell::Literal(_, l)
                | blocks::Cell::Blob(_, l)
                | blocks::Cell::Link(_, _, l) => *l,
                blocks::Cell::LiteralIncomplete(_, l) | blocks::Cell::BlobIncomplete(_, l) => {
                    l.identifier
                }
            } == current_object
            {
                println!(
                    "{}",
                    match k {
                        blocks::Cell::Literal(m, _) =>
                            format!("{} : [Literal] {}", current_field, m),
                        blocks::Cell::Blob(m, _) =>
                            format!("{}: [Blob] {}", current_field, hex::encode(m)),
                        blocks::Cell::Link(m, n, _) => {
                            format!(
                                "{}: [Link] {} - {}",
                                current_field,
                                match m {
                                    blocks::LinkType::Forward => "Forward",
                                    blocks::LinkType::Reverse => "Reverse",
                                },
                                match n {
                                    blocks::LinkTarget::SameBlock(o) => into_hex_block(*o),
                                    blocks::LinkTarget::AnotherField(o, p) =>
                                        format!("{}/{}", into_hex_metadata(*o), into_hex_block(*p)),
                                    blocks::LinkTarget::AnotherStruct(o, p, q) => format!(
                                        "{}/{}/{}",
                                        into_hex_metadata(*o),
                                        into_hex_metadata(*p),
                                        into_hex_block(*q)
                                    ),
                                }
                            )
                        }
                        blocks::Cell::BlobIncomplete(m, _) =>
                            format!("{}: [BlobIncomplete] {}", current_field, hex::encode(m)),
                        blocks::Cell::LiteralIncomplete(m, _) =>
                            format!("{}: [LiteralIncomplete] {}", current_field, hex::encode(m)),
                    }
                )
            }
        }
    }
}

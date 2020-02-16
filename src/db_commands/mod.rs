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
/// Operate database commands
pub fn run_commands(
    argument: &str,
    //metadata_cache: &mut HashMap<[u8; METADATA_INDEX_LEN as usize], metadata::Metadata>,
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
                                    structure_cache,
                                );
                            }
                        }
                    },
                    j => match current_location.current_structure_identifier() {
                        None => println!("You cannot create an object or a cell in root."),
                        Some(_) => {
                            if current_location.current_object_identifier().is_none() {
                                //创建一个新的Object
                                let object_name = j;
                                //检查Object是否存在
                                let is_exist = {
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
                                    //TODO: 想办法优化这段代码
                                    for k in cells {
                                        if let blocks::Cell::Literal(l, _) = k {
                                            if !object_exist && object_name == l {
                                                object_exist = true;
                                            }
                                        }
                                    }
                                    object_exist
                                };
                                if is_exist {
                                    println!("Object already exists.");
                                } else {
                                    create_object(j, current_location, structure_cache);
                                }
                            } else {
                                //新的Cell
                                //首先判断是否已经存在Field
                                let field_identifier = if let Some(k) = structure_cache
                                    .get(
                                        &current_location
                                            .current_structure_identifier()
                                            .expect("Unable to find current structure identifier"),
                                    )
                                    .expect("Unable to read structure cache metadata")
                                    .metadata
                                    .sub_data()
                                    .get(j)
                                {
                                    println!("Using existing field {}[{}]", j, k,);
                                    let result = k;
                                    result.to_string()
                                } else {
                                    //创建一个新的Field
                                    //TODO: 确保field name != list
                                    into_hex_metadata(create_field(
                                        j,
                                        current_location,
                                        structure_cache,
                                    ))
                                };
                                let cell_type = {
                                    if let Some(k) = parsed_command.next() {
                                        let k = k.to_ascii_lowercase();
                                        match k.as_str() {
                                            //TODO: 添加Incomplete Block支持
                                            "literal" => blocks::CellType::Literal,
                                            "blob" => blocks::CellType::Blob,
                                            "link" => blocks::CellType::Link,
                                            "revlink" => blocks::CellType::ReverseLink,
                                            _ => {
                                                println!("Warning: cell type not reconized. Treated as Literal Cell.");
                                                blocks::CellType::Literal
                                            }
                                        }
                                    } else {
                                        println!("Warning: creating a cell with no type is discouraged. Treated as Literal Cell.");
                                        blocks::CellType::Literal
                                    }
                                };
                                let cell_content = {
                                    if let Some(k) = parsed_command.next() {
                                        k
                                    } else {
                                        println!("Warning: creating a cell with no content is discouraged.");
                                        "null"
                                    }
                                };
                                let result_cell = match cell_type {
                                    blocks::CellType::Literal => blocks::Cell::Literal(
                                        cell_content.to_string(),
                                        current_location.current_object_identifier().unwrap(),
                                    ),
                                    blocks::CellType::Blob => blocks::Cell::Blob(
                                        binary_io::read_all(cell_content),
                                        current_location.current_object_identifier().unwrap(),
                                    ),
                                    blocks::CellType::Link | blocks::CellType::ReverseLink => {
                                        let link_target: Vec<&str> =
                                            cell_content.split('/').collect();
                                        blocks::Cell::Link(
                                            if let blocks::CellType::Link = cell_type {
                                                blocks::LinkType::Forward
                                            } else {
                                                blocks::LinkType::Reverse
                                            },
                                            match link_target.len() {
                                                1 => blocks::LinkTarget::SameBlock(
                                                    from_hex_blocks(&link_target[0].to_string()),
                                                ),
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
                                };
                                //将cell_content放到cache里
                                if structure_cache
                                    .get(&current_location.current_structure_identifier().unwrap())
                                    .unwrap()
                                    .cached_block
                                    .get(&from_hex_metadata(&field_identifier))
                                    .is_none()
                                {
                                    //先缓存
                                    println!("Please cache the cell before writing");
                                } else {
                                    create_cell(
                                        &field_identifier,
                                        result_cell,
                                        current_location,
                                        structure_cache,
                                    );
                                }
                            }
                        }
                    },
                },
            };
        }
        "leave" => {
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
        "structure_cache" => {
            for i in structure_cache {
                println!("{} - {:?}", into_hex_metadata(*i.0), *i.0);
            }
        }
        "sync" => {
            println!("Writing main metadata");
            {
                let main_metadata_vec = main_metadata.to_vec();
                let data = blockencrypt::encrypt_block(&main_metadata_vec, password);
                let filename = format!("{}/metadata", current_location.root_folder().unwrap());
                binary_io::write_with_nonce(&filename, &data.0, data.1);
            }
            for i in structure_cache {
                println!("Writing {}", into_hex_metadata(*i.0));
                {
                    println!(" Cell List");
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
                    println!(" Cache");
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
                            let filename = format!("{}/{}", folder_name, current_num);
                            binary_io::write_with_nonce(&filename, &data.0, data.1);
                        }
                    }
                }
                {
                    println!(" Metadata");
                    //Save metadata
                    let metadata_sync_vec = i.1.metadata.to_vec();
                    let data = blockencrypt::encrypt_block(&metadata_sync_vec, password);
                    let filename = format!(
                        "{}/{}/metadata",
                        current_location.root_folder().unwrap(),
                        into_hex_metadata(*i.0)
                    );
                    binary_io::write_with_nonce(&filename, &data.0, data.1);
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
        "ls" => {
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
fn create_structure(
    structure_name: &str,
    password: &secretbox::Key,
    main_metadata: &mut metadata::Metadata,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
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
    //Create structure cache
    {
        let mut structure_data = metadata::Metadata::create();
        structure_data.new_attribute(&String::from("type"), &String::from("struct"));
        //TODO: size可变
        structure_data.new_attribute(&String::from("size"), &String::from("32"));
        //Create a identifier for list
        let list_identifier = random_metadata_identifier();
        structure_data.new_sub_data(&String::from("list"), &into_hex_metadata(list_identifier));
        //将Metadata导出
        let metadata_vec = structure_data.to_vec();
        let data = blockencrypt::encrypt_block(&metadata_vec, password);
        let filename = format!(
            "{}/{}/metadata",
            current_location.root_folder().unwrap(),
            into_hex_metadata(structure_token)
        );
        binary_io::write_with_nonce(&filename, &data.0, data.1);
        //封装structure
        let result_struct = Structure {
            metadata: structure_data,
            list: blocks::BlockQueue::new(),
            cached_block: HashMap::new(),
        };
        structure_cache.insert(structure_token, result_struct);
    }
    println!(
        "New structure {}[{}]",
        structure_name,
        into_hex_metadata(structure_token)
    );
}
fn create_object(
    object_name: &str,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    //Some
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
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .list
        .import_cell(blocks::Cell::Literal(
            object_name.to_string(),
            object_identifier,
        ));
    println!(
        "New structure {}[{}]",
        object_name,
        into_hex_block(object_identifier)
    );
}
fn create_field(
    field_name: &str,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) -> [u8; METADATA_INDEX_LEN as usize] {
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
    //将Field写入Metadata中
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .expect("Unable to read structure cache metadata")
        .metadata
        .new_sub_data(
            &field_name.to_string(),
            &into_hex_metadata(field_identifier),
        );
    //创建Field
    fs::create_dir(format!(
        "{}/{}/{}",
        current_location.root_folder().unwrap(),
        into_hex_metadata(current_location.current_structure_identifier().unwrap()),
        into_hex_metadata(field_identifier)
    ))
    .expect("Unable to create directory");
    //封装Field
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
        .insert(field_identifier, blocks::BlockQueue::new());
    println!(
        "New field {}[{}]",
        field_name,
        into_hex_metadata(field_identifier)
    );
    field_identifier
}
fn create_cell(
    field_identifier: &str,
    cell: blocks::Cell,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    structure_cache
        .get_mut(&current_location.current_structure_identifier().unwrap())
        .unwrap()
        .cached_block
        .get_mut(&from_hex_metadata(field_identifier))
        .unwrap()
        .import_cell(cell);
}
fn select_structure(
    structure_name: &str,
    password: &secretbox::Key,
    main_metadata: &mut metadata::Metadata,
    current_location: &mut DatabaseLocation,
    structure_cache: &mut HashMap<[u8; 8], Structure>,
) {
    let structure_token = main_metadata
        .sub_data()
        .get(structure_name)
        .expect("Unable to find target structure in main metadata cache");
    let structure_list_path = format!(
        "{}/{}/metadata",
        current_location.root_folder().unwrap(),
        structure_token
    );
    let mut structure_metadata = metadata::Metadata::create();
    let structure_list_raw = binary_io::read_with_nonce(&structure_list_path);
    let structure_list_vec =
        blockencrypt::decrypt_block(&structure_list_raw.0, password, structure_list_raw.1);
    structure_metadata.import(structure_list_vec);
    let cell_list = {
        //读取cached_block
        let block_list_path = format!(
            "{}/{}/{}",
            current_location.root_folder().unwrap(),
            structure_token,
            structure_metadata
                .sub_data()
                .get(&String::from("list"))
                .unwrap()
        );
        let mut final_block_list = blocks::BlockQueue::new();
        if path::Path::new(&block_list_path).is_file() {
            let block_list_raw = binary_io::read_with_nonce(&block_list_path);
            let block_list_vec =
                blockencrypt::decrypt_block(&block_list_raw.0, password, block_list_raw.1);
            final_block_list.import_from_vec(block_list_vec);
            final_block_list.raw_to_cell(
                structure_metadata
                    .attribute()
                    .get(&String::from("size"))
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
        }
        final_block_list
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
    println!("Selected structure {}[{}]", structure_name, structure_token);
}
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
        id.expect("Unable to find object")
    };
    current_location.select_object((object_identifier, object_name.to_string()));
}
fn clear_cache(structure_cache: &mut HashMap<[u8; 8], Structure>) {
    for i in structure_cache {
        println!("Structure {}", into_hex_metadata(*i.0));
        for j in &mut i.1.cached_block {
            j.1.clean_cells();
        }
    }
}

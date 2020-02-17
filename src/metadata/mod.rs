use std::collections::HashMap;
pub struct Metadata {
    has_modified: bool,
    attribute: HashMap<String, String>,
    sub_data: HashMap<String, String>,
}
impl Metadata {
    pub const fn attribute(&self) -> &HashMap<String, String> {
        &self.attribute
    }
    pub const fn sub_data(&self) -> &HashMap<String, String> {
        &self.sub_data
    }
    pub const fn has_modified(&self) -> bool {
        self.has_modified
    }
    pub fn into_vec(self) -> Vec<u8> {
        let mut index: Vec<u8> = Vec::new();
        for (key, val) in self.attribute {
            index.append(&mut key.into_bytes());
            index.append(&mut String::from("=").as_bytes().to_vec());
            index.append(&mut val.into_bytes());
            index.append(&mut String::from(";").as_bytes().to_vec());
        }
        index.append(&mut String::from("$").as_bytes().to_vec());
        for (key, val) in self.sub_data {
            index.append(&mut key.into_bytes());
            index.append(&mut String::from("=").as_bytes().to_vec());
            index.append(&mut val.into_bytes());
            index.append(&mut String::from(";").as_bytes().to_vec());
        }
        index
    }
    pub fn from_vec(metadata_block: Vec<u8>) -> Self {
        let mut result = Self::create();
        result.import(metadata_block);
        result
    }
    pub fn to_vec(&self) -> Vec<u8> {
        let mut index: Vec<u8> = Vec::new();
        for (key, val) in &self.attribute {
            index.append(&mut key.clone().into_bytes());
            index.append(&mut String::from("=").as_bytes().to_vec());
            index.append(&mut val.clone().into_bytes());
            index.append(&mut String::from(";").as_bytes().to_vec());
        }
        index.append(&mut String::from("$").as_bytes().to_vec());
        for (key, val) in &self.sub_data {
            index.append(&mut key.clone().into_bytes());
            index.append(&mut String::from("=").as_bytes().to_vec());
            index.append(&mut val.clone().into_bytes());
            index.append(&mut String::from(";").as_bytes().to_vec());
        }
        index
    }
    pub fn new_attribute(&mut self, lhs: &str, rhs: &str) {
        self.attribute
            .entry(lhs.to_string())
            .or_insert_with(|| rhs.to_string());
        if !self.has_modified {
            self.has_modified = true;
        }
    }
    pub fn new_sub_data(&mut self, lhs: &str, rhs: &str) {
        self.sub_data
            .entry(lhs.to_string())
            .or_insert_with(|| rhs.to_string());
        if !self.has_modified {
            self.has_modified = true;
        }
    }
    pub fn import(&mut self, metadata_block: Vec<u8>) {
        let metadata_block =
            String::from_utf8(metadata_block).expect("Unable to convert Metadata Vector to String");
        println!("{}", metadata_block);
        let mut attribute_and_data = metadata_block.split('$');
        let attribute_iter = attribute_and_data
            .next()
            .expect("Unable to load attribute from import")
            .split(';');
        let data_iter = attribute_and_data
            .next()
            .expect("Unable to load data from import")
            .split(';');
        for current_token in attribute_iter {
            if current_token != "" {
                let mut token_parse = current_token.split('=');
                let lhs = token_parse.next().unwrap();
                let rhs = token_parse.next().unwrap();
                self.attribute.insert(lhs.to_string(), rhs.to_string());
            }
        }
        for current_token in data_iter {
            if current_token != "" {
                let mut token_parse = current_token.split('=');
                let lhs = token_parse.next().unwrap();
                let rhs = token_parse.next().unwrap();
                self.sub_data.insert(lhs.to_string(), rhs.to_string());
            }
        }
        if !self.has_modified {
            self.has_modified = true;
        }
    }
    pub fn clear(&mut self) {
        self.attribute = HashMap::new();
        self.sub_data = HashMap::new();
        if !self.has_modified {
            self.has_modified = true;
        }
    }
    pub fn create() -> Self {
        Self {
            attribute: HashMap::new(),
            sub_data: HashMap::new(),
            has_modified: false,
        }
    }
}
impl std::fmt::Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let r1 = "Attribute(s):\n";
        let mut r2 = String::new();
        for (i, j) in &self.attribute {
            let t1 = format!(" {} = {}", i, j);
            r2.push_str(&t1);
            //write!(f, " {} = {}\n", i, j);
        }
        let r3 = "\nData(s):\n";
        let mut r4 = String::new();
        for (i, j) in &self.sub_data {
            let t1 = format!(" {} = {}", i, j);
            r4.push_str(&t1);
        }
        write!(f, "{}{}{}{}", r1, r2, r3, r4)
    }
}

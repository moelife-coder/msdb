pub const CELL_IDENTIFIER_LENGTH: u32 = 8;
use super::db_commands::METADATA_INDEX_LEN;
use std::collections::HashMap;
use std::convert::TryInto;
pub enum Cell {
    Literal(String, [u8; CELL_IDENTIFIER_LENGTH as usize]),
    Blob(Vec<u8>, [u8; CELL_IDENTIFIER_LENGTH as usize]),
    Link(LinkType, LinkTarget, [u8; CELL_IDENTIFIER_LENGTH as usize]),
    LiteralIncomplete(Vec<u8>, IncompleteIdentifier),
    BlobIncomplete(Vec<u8>, IncompleteIdentifier),
}
impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Literal(i, j) => write!(f, "[Literal]({}) {}", hex::encode(j), i),
            Self::Blob(i, j) => write!(f, "[Blob]({}) {:?}", hex::encode(j), i),
            Self::Link(i, j, k) => write!(
                f,
                "{}",
                format!(
                    "[{}]({}) {}",
                    match i {
                        LinkType::Forward => "ForwardLink",
                        LinkType::Reverse => "ReverseLink",
                    },
                    hex::encode(k),
                    match j {
                        LinkTarget::SameBlock(l) => hex::encode(l),
                        LinkTarget::AnotherField(l, m) =>
                            format!("{} -> {}", hex::encode(l), hex::encode(m)),
                        LinkTarget::AnotherStruct(l, m, n) => format!(
                            "{} -> {} -> {}",
                            hex::encode(l),
                            hex::encode(m),
                            hex::encode(n)
                        ),
                    }
                )
            ),
            Self::LiteralIncomplete(i, j) => write!(
                f,
                "[LiteralIncomplete]({} - {}{}) {}",
                hex::encode(j.identifier),
                if j.is_final { "F" } else { "" },
                format!("{}", j.num),
                hex::encode(i)
            ),
            Self::BlobIncomplete(i, j) => write!(
                f,
                "[BlobIncomplete]({} - {}{}) {}",
                hex::encode(j.identifier),
                if j.is_final { "F" } else { "" },
                format!("{}", j.num),
                hex::encode(i)
            ),
        }
    }
}
pub enum LinkTarget {
    SameBlock([u8; CELL_IDENTIFIER_LENGTH as usize]),
    AnotherField(
        [u8; METADATA_INDEX_LEN as usize],
        [u8; CELL_IDENTIFIER_LENGTH as usize],
    ),
    AnotherStruct(
        [u8; METADATA_INDEX_LEN as usize],
        [u8; METADATA_INDEX_LEN as usize],
        [u8; CELL_IDENTIFIER_LENGTH as usize],
    ),
}
pub enum LinkType {
    Forward,
    Reverse,
}
pub struct IncompleteIdentifier {
    pub identifier: [u8; 8],
    num: u8,
    is_final: bool,
}
pub struct BlockQueue {
    pub queue: Vec<Vec<u8>>,
    pub cells: Vec<Cell>,
}
impl std::fmt::Display for BlockQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut cached_block_string = String::from("Queue:\n");
        let mut queue_count = 0;
        for i in &self.queue {
            cached_block_string.push_str(&format!("[{}]: ", queue_count));
            cached_block_string.push_str(&format!("{}\n", hex::encode(i)));
            queue_count += 1;
        }
        cached_block_string.push_str("Cells:\n");
        for i in &self.cells {
            cached_block_string.push_str(&format!("[{}]: ", queue_count));
            cached_block_string.push_str(&format!("{}", i));
        }
        write!(f, "{}", cached_block_string)
    }
}
struct CellReadingBuffer {
    cell_size: u32,
    cell_opcode: u8,
    current_byte_offset: u32,
    identifier: [u8; 8],
    content: Vec<u8>,
}
impl CellReadingBuffer {
    const fn new() -> Self {
        Self {
            cell_size: 0,
            cell_opcode: 0,
            current_byte_offset: 0,
            identifier: [0, 0, 0, 0, 0, 0, 0, 0],
            content: Vec::new(),
        }
    }
}
impl BlockQueue {
    pub const fn new() -> Self {
        Self {
            queue: Vec::new(),
            cells: Vec::new(),
        }
    }
    pub fn import_from_vec(&mut self, raw_content: Vec<u8>) {
        self.queue.push(raw_content);
    }
    pub fn from_vec(raw_content: Vec<u8>, default_cell_size: u32) -> Self {
        let mut result = Self::new();
        result.import_from_vec(raw_content);
        result.raw_to_cell(default_cell_size);
        result
    }
    pub fn raw_to_cell(&mut self, default_cell_size: u32) {
        let mut pre_translate_result: Vec<CellReadingBuffer> = Vec::new();
        for i in &mut self.queue {
            let mut current_cell = CellReadingBuffer::new();
            let mut block_offset: u32 = 0;
            let mut is_ended: bool = false;
            let mut same_size_as_default: bool = true;
            //在offset = len之前不停读取
            while (block_offset as usize) < i.len() {
                match current_cell.current_byte_offset {
                    0 => {
                        //第一Byte
                        //1. 判断是否需要对cell_size进行对齐修改
                        //2. 识别出OPCODE
                        current_cell.cell_opcode =
                            i[(current_cell.current_byte_offset + block_offset) as usize];
                        match i[(current_cell.current_byte_offset + block_offset) as usize] % 2 {
                            0 => {
                                //size和default一样
                                current_cell.cell_opcode -= 1;
                            }
                            1 => {
                                //size和default不一样
                                same_size_as_default = false;
                            }
                            _ => panic!("Error when trying to find opcode and size for a cell."),
                        }
                    }
                    1..=12 => {
                        if !same_size_as_default {
                            //i[1]~i[4] => 大小
                            //i[5]~i[12] => Identifier
                            match current_cell.current_byte_offset {
                                1..=4 => {
                                    current_cell.cell_size += u32::from(
                                        i[(current_cell.current_byte_offset + block_offset)
                                            as usize],
                                    ) * 256_u32
                                        .pow(4 - current_cell.current_byte_offset)
                                        as u32;
                                }
                                5..=12 => {
                                    current_cell.identifier
                                        [current_cell.current_byte_offset as usize - 5] = i
                                        [(current_cell.current_byte_offset + block_offset)
                                            as usize];
                                }
                                _ => panic!(
                                    "Error when loading byte offset for a different size cell"
                                ),
                            }
                        } else {
                            //i[1]~i[8]  => identifier
                            if let 1..=8 = current_cell.current_byte_offset {
                                current_cell.cell_size = default_cell_size;
                                current_cell.identifier
                                    [current_cell.current_byte_offset as usize - 1] =
                                    i[(current_cell.current_byte_offset + block_offset) as usize];
                            } else {
                                if current_cell.current_byte_offset == current_cell.cell_size + 8 {
                                    is_ended = true;
                                }
                                current_cell.content.push(
                                    i[(current_cell.current_byte_offset + block_offset) as usize],
                                );
                            }
                        }
                    }
                    _ => {
                        //读取内容，直到最后一Byte
                        //首先检查剩余Byte数
                        if current_cell.current_byte_offset
                            == current_cell.cell_size + if same_size_as_default { 8 } else { 12 }
                        {
                            //目前的东西是最后一byte
                            is_ended = true;
                        }
                        //读取内容
                        current_cell
                            .content
                            .push(i[(current_cell.current_byte_offset + block_offset) as usize])
                    }
                }
                if is_ended {
                    //将byte_offset叠加
                    block_offset +=
                        current_cell.cell_size + if same_size_as_default { 9 } else { 13 };
                    //将current_cell push到result中
                    pre_translate_result.push(current_cell);
                    current_cell = CellReadingBuffer::new();
                    is_ended = false;
                    same_size_as_default = true;
                } else {
                    current_cell.current_byte_offset += 1;
                }
            }
        }
        for i in pre_translate_result {
            match i.cell_opcode {
                //可能的OPCODE:
                //1 - Literal
                //3 - Blob
                //5 - Link(Same Block + Forward)
                //7 - Link(Different Block + Forward)
                //9 - Link(Different Struct + Forward)
                //11 - Link(Same Block + Reverse)
                //13 - Link(Different Block + Reverse)
                //15 - Link(Different Struct + Reverse)
                //17 - Continue(Init - Text)
                //19 - Continue(Init - Blob)
                //21 - Continue
                0 => panic!(
                    "Found OPCODE - 0 in {} raw block",
                    String::from_utf8(i.identifier.to_vec()).expect(
                        "Unable to decode identifier to string when warning OPCODE failure"
                    )
                ),
                1 => self.cells.push(Cell::Literal(
                    String::from_utf8(i.content)
                        .expect("Unable to read from literal cell to utf8 string"),
                    i.identifier,
                )),
                3 => self.cells.push(Cell::Blob(i.content, i.identifier)),
                5 => {
                    //一共8 Byte
                    let result_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [0..CELL_IDENTIFIER_LENGTH as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Forward,
                        LinkTarget::SameBlock(result_identifier),
                        i.identifier,
                    ));
                }
                7 => {
                    //一共16 Byte
                    let result_field_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [0..METADATA_INDEX_LEN as usize]
                        .try_into()
                        .unwrap();
                    let result_cell_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + CELL_IDENTIFIER_LENGTH) as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Forward,
                        LinkTarget::AnotherField(result_field_identifier, result_cell_identifier),
                        i.identifier,
                    ));
                }
                9 => {
                    //一共24 Byte
                    let result_struct_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [0..METADATA_INDEX_LEN as usize]
                        .try_into()
                        .unwrap();
                    let result_field_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + METADATA_INDEX_LEN) as usize]
                        .try_into()
                        .unwrap();
                    let result_cell_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + CELL_IDENTIFIER_LENGTH) as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Forward,
                        LinkTarget::AnotherStruct(
                            result_struct_identifier,
                            result_field_identifier,
                            result_cell_identifier,
                        ),
                        i.identifier,
                    ));
                }
                11 => {
                    //一共8 Byte
                    let result_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [0..CELL_IDENTIFIER_LENGTH as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Reverse,
                        LinkTarget::SameBlock(result_identifier),
                        i.identifier,
                    ));
                }
                13 => {
                    //一共16 Byte
                    let result_field_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [0..METADATA_INDEX_LEN as usize]
                        .try_into()
                        .unwrap();
                    let result_cell_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + CELL_IDENTIFIER_LENGTH) as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Reverse,
                        LinkTarget::AnotherField(result_field_identifier, result_cell_identifier),
                        i.identifier,
                    ));
                }
                15 => {
                    let result_struct_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [0..METADATA_INDEX_LEN as usize]
                        .try_into()
                        .unwrap();
                    let result_field_identifier: [u8; METADATA_INDEX_LEN as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + METADATA_INDEX_LEN) as usize]
                        .try_into()
                        .unwrap();
                    let result_cell_identifier: [u8; CELL_IDENTIFIER_LENGTH as usize] = i.content
                        [METADATA_INDEX_LEN as usize
                            ..(METADATA_INDEX_LEN + CELL_IDENTIFIER_LENGTH) as usize]
                        .try_into()
                        .unwrap();
                    self.cells.push(Cell::Link(
                        LinkType::Reverse,
                        LinkTarget::AnotherStruct(
                            result_struct_identifier,
                            result_field_identifier,
                            result_cell_identifier,
                        ),
                        i.identifier,
                    ));
                }
                17 => self.cells.push(Cell::LiteralIncomplete(
                    i.content,
                    IncompleteIdentifier {
                        identifier: i.identifier,
                        num: 0,
                        is_final: false,
                    },
                )),
                19 => self.cells.push(Cell::BlobIncomplete(
                    i.content,
                    IncompleteIdentifier {
                        identifier: i.identifier,
                        num: 0,
                        is_final: false,
                    },
                )),
                21 => {
                    //第一Byte是序列号, 第二字节是is_final(1为真，0为否)
                    let num: u8 = i.content[0];
                    let is_final: bool = {
                        match i.content[1] {
                            0 => false,
                            1 => true,
                            _ => panic!("Error when parsing extended data"),
                        }
                    };
                    let mut cell_content = i.content;
                    cell_content.remove(0);
                    cell_content.remove(0);
                    let mut identifier_list: (Vec<u8>, bool, bool) = (vec![num], false, false);
                    if is_final {
                        identifier_list.1 = true;
                    } else {
                        for j in &self.cells {
                            if match j {
                                Cell::BlobIncomplete(_, k) | Cell::LiteralIncomplete(_, k) => {
                                    k.identifier
                                }
                                _ => [0; CELL_IDENTIFIER_LENGTH as usize],
                            } == i.identifier
                            {
                                identifier_list.0.push(match j {
                                    Cell::BlobIncomplete(_, k) | Cell::LiteralIncomplete(_, k) => {
                                        k.num
                                    }
                                    _ => 0,
                                });
                                if match j {
                                    Cell::BlobIncomplete(_, k) | Cell::LiteralIncomplete(_, k) => {
                                        k.is_final
                                    }
                                    _ => false,
                                } {
                                    identifier_list.1 = true;
                                }
                            }
                        }
                    };
                    if identifier_list.1 {
                        identifier_list.2 = true;
                        identifier_list.0.sort();
                        for j in 0..identifier_list.0.len() - 1 {
                            if identifier_list.0[j] + 1 != identifier_list.0[j + 1] {
                                identifier_list.2 = false;
                            }
                        }
                    }
                    if identifier_list.2 {
                        let mut collapse_incomplete: HashMap<u8, &Cell> = HashMap::new();
                        let mut content_result: Vec<u8> = Vec::new();
                        let mut maxium_num: u8 = 0;
                        let mut is_literal_cell = true;
                        for j in &self.cells {
                            if match j {
                                Cell::BlobIncomplete(_, k) | Cell::LiteralIncomplete(_, k) => {
                                    k.identifier
                                }
                                _ => [0; CELL_IDENTIFIER_LENGTH as usize],
                            } == i.identifier
                            {
                                if let Cell::BlobIncomplete(_, _) = j {
                                    is_literal_cell = false;
                                }
                                collapse_incomplete.insert(
                                    match j {
                                        Cell::BlobIncomplete(_, k)
                                        | Cell::LiteralIncomplete(_, k) => k.num,
                                        _ => 0,
                                    },
                                    j,
                                );
                                if match j {
                                    Cell::BlobIncomplete(_, k) | Cell::LiteralIncomplete(_, k) => {
                                        k.num
                                    }
                                    _ => 0,
                                } > maxium_num
                                {
                                    maxium_num = match j {
                                        Cell::BlobIncomplete(_, k)
                                        | Cell::LiteralIncomplete(_, k) => k.num,
                                        _ => 0,
                                    };
                                }
                            }
                        }
                        for j in 0..=maxium_num {
                            if j == num {
                                for l in &cell_content {
                                    content_result.push(*l);
                                }
                            } else {
                                let k = match collapse_incomplete.get(&j).unwrap() {
                                    Cell::BlobIncomplete(l, _) | Cell::LiteralIncomplete(l, _) => l,
                                    _ => panic!("Unexpected incomplete type"),
                                };
                                for l in k {
                                    content_result.push(*l);
                                }
                            }
                        }
                        self.cells.push(if is_literal_cell {
                            Cell::Literal(
                                String::from_utf8(content_result).expect(
                                    "Unable to read from multiple literal cell to utf8 string",
                                ),
                                i.identifier,
                            )
                        } else {
                            Cell::Blob(content_result, i.identifier)
                        });
                    } else {
                        let mut is_literal = true;
                        for j in &self.cells {
                            if match j {
                                Cell::BlobIncomplete(_, k) => k.identifier,
                                _ => [0; CELL_IDENTIFIER_LENGTH as usize],
                            } == i.identifier
                            {
                                is_literal = false;
                            }
                        }
                        self.cells.push(if is_literal {
                            Cell::LiteralIncomplete(
                                cell_content,
                                IncompleteIdentifier {
                                    identifier: i.identifier,
                                    num,
                                    is_final,
                                },
                            )
                        } else {
                            Cell::BlobIncomplete(
                                cell_content,
                                IncompleteIdentifier {
                                    identifier: i.identifier,
                                    num,
                                    is_final,
                                },
                            )
                        })
                    }
                }
                _ => {}
            }
        }
    }
    pub fn cell_to_raw(&mut self, vector_length: Option<u32>, default_cell_size: u32) {
        let mut pre_translate_result: Vec<CellReadingBuffer> = Vec::new();
        let vector_length = if let Some(i) = vector_length {
            i
        } else {
            u32::max_value()
        };
        for i in &self.cells {
            let mut current_buffer = CellReadingBuffer::new();
            current_buffer.identifier = match i {
                Cell::Blob(_, j) | Cell::Link(_, _, j) | Cell::Literal(_, j) => *j,
                Cell::BlobIncomplete(_, j) | Cell::LiteralIncomplete(_, j) => j.identifier,
            };
            match &i {
                Cell::Literal(j, _) => {
                    current_buffer.cell_size = j.len().try_into().unwrap();
                    current_buffer.cell_opcode = 1;
                    current_buffer.content = j.as_bytes().to_vec();
                }
                Cell::Blob(j, _) => {
                    current_buffer.cell_size = j.len().try_into().unwrap();
                    current_buffer.cell_opcode = 3;
                    current_buffer.content = j.to_vec();
                }
                Cell::Link(j, k, _) => {
                    match k {
                        LinkTarget::SameBlock(k) => {
                            current_buffer.cell_opcode = 5;
                            current_buffer.cell_size = 8;
                            current_buffer.content = k.to_vec();
                        }
                        LinkTarget::AnotherField(k, l) => {
                            current_buffer.cell_opcode = 7;
                            current_buffer.cell_size = 16;
                            current_buffer.content = k.to_vec();
                            for m in l {
                                current_buffer.content.push(*m);
                            }
                        }
                        LinkTarget::AnotherStruct(k, l, m) => {
                            current_buffer.cell_opcode = 9;
                            current_buffer.cell_size = 24;
                            current_buffer.content = k.to_vec();
                            for n in l {
                                current_buffer.content.push(*n);
                            }
                            for n in m {
                                current_buffer.content.push(*n);
                            }
                        }
                    }
                    if let LinkType::Reverse = j {
                        current_buffer.cell_opcode += 6;
                    }
                }
                Cell::LiteralIncomplete(j, k) | Cell::BlobIncomplete(j, k) => {
                    if k.num == 0 {
                        current_buffer.cell_opcode = if let Cell::LiteralIncomplete(_, _) = &i {
                            13
                        } else {
                            15
                        };
                        current_buffer.cell_size = j.len().try_into().unwrap();
                        current_buffer.content = j.to_vec();
                    } else {
                        current_buffer.cell_opcode = 17;
                        current_buffer.cell_size = (j.len() + 2).try_into().unwrap();
                        current_buffer.content = vec![k.num, if k.is_final { 1 } else { 0 }];
                        for l in j {
                            current_buffer.content.push(*l);
                        }
                    }
                }
            }
            pre_translate_result.push(current_buffer);
        }
        let mut result_vec: Vec<u8> = Vec::new();
        for i in &mut pre_translate_result {
            if result_vec.len() + 5 + i.identifier.len() + i.content.len()
                > (vector_length as usize)
            {
                self.queue.push(result_vec);
                result_vec = Vec::new();
            }
            //判断大小是否是默认大小
            let mut first_byte: u8 = i.cell_opcode;
            if i.cell_size == default_cell_size {
                first_byte += 1;
                result_vec.push(first_byte);
            } else {
                //计算大小
                result_vec.push(first_byte);
                for j in 0..4 {
                    result_vec.push((i.cell_size / 256_u32.pow(3 - j)).try_into().unwrap());
                    i.cell_size -= (i.cell_size / 256_u32.pow(3 - j)) * (256_u32.pow(3 - j));
                }
            }
            for j in &i.identifier {
                result_vec.push(*j);
            }
            //附上内容
            for j in &i.content {
                result_vec.push(*j);
            }
        }
        if !result_vec.is_empty() {
            self.queue.push(result_vec);
        }
    }
    pub fn clean_cells(&mut self) {
        self.cells = Vec::new();
    }
    pub fn import_cell(&mut self, cell: Cell) {
        self.cells.push(cell);
    }
    pub fn delete_cell(&mut self, identifier: [u8; CELL_IDENTIFIER_LENGTH as usize]) {
        let mut position_deletd = Vec::new();
        let mut position = 0;
        for i in &self.cells {
            if match i {
                Cell::Blob(_, j) | Cell::Literal(_, j) | Cell::Link(_, _, j) => *j,
                Cell::BlobIncomplete(_, j) | Cell::LiteralIncomplete(_, j) => j.identifier,
            } == identifier
            {
                position_deletd.push(position);
            } else {
                position += 1;
            }
        }
        for i in position_deletd {
            self.cells.remove(i);
        }
    }
    pub fn delete_literal_cell_based_on_content(&mut self, content: &str) {
        let mut position_deletd = Vec::new();
        let mut position = 0;
        for i in &self.cells {
            if match i {
                Cell::Literal(j, _) => j,
                _ => "",
            } == content
            {
                position_deletd.push(position);
            } else {
                position += 1;
            }
        }
        for i in position_deletd {
            self.cells.remove(i);
        }
    }
}

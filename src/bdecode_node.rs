mod utils;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use bitfields::bitfield;

use crate::{
    limits::{self, BUFFER_MAX_OFFSET, DEFAULT_DEPTH_LIMIT, DEFAULT_TOKEN_LIMIT},
    utils::{check_integer, parse_uint},
    BdecodeError, BdecodeResult, BdecodeToken, BdecodeTokenType,
};

/// 解析 dict 或 list 过程中的会生成一个 stack, 
/// 其中每一个 StackFrame 元素都对应 dict 或 list 的入口 token .
#[bitfield(u32)]
#[derive(Clone, Copy)]
struct StackFrame {
    #[bits(31)]
    token: u32,

    #[bits(1)]
    state: u8,
}

/// 用于存放解析后的数据
#[derive(Clone)]
pub struct BdecodeNode {
    /// 当前节点在 tokens 中的索引
    /// 0 - root 节点值; -1 - 未初始化  
    pub token_index: u32,

    /// 存放 list 和 map 中 item 的对应的 token 索引集合
    pub item_indexes: Arc<Vec<u32>>,
    // pub item_begin_len: Option<(u32, u32)>,

    /// list 和 map 中 item 的数量
    len: usize,

    /// 解析后的 token 集合
    pub tokens: Arc<Vec<BdecodeToken>>,

    /// 存放解析前字符串的 buffer
    pub buffer: Arc<Vec<u8>>,
}

impl BdecodeNode {
    pub fn with_buffer(buffer: Arc<Vec<u8>>) -> BdecodeResult<Self> {
        Self::new(buffer, None, None)
    }

    pub fn new(
        buffer: Arc<Vec<u8>>,
        depth_limit: Option<usize>,
        token_limit: Option<i32>,
    ) -> BdecodeResult<Self> {
        let depth_limit = depth_limit.unwrap_or(DEFAULT_DEPTH_LIMIT);
        let mut token_limit = token_limit.unwrap_or(DEFAULT_TOKEN_LIMIT as i32);

        let mut tokens = Vec::<BdecodeToken>::new();

        if buffer.len() > BUFFER_MAX_OFFSET as usize {
            Err(BdecodeError::LimitExceeded(buffer.len()))?
        }

        let mut start = 0;
        let end = buffer.len();

        // stack 在解析 dict 和 list 时才会使用。
        // 它的每一项都是存放的 dict 和 list 本身的入口 token 对应的 frame （注意不是 dict 和 list 的第一个元素的 token ）。
        let mut stack = Vec::<StackFrame>::with_capacity(depth_limit as usize);

        // current_frame_ptr 只会指向已处理完的 frame , 不会指向处理一半的 frame 。
        let mut current_frame_ptr: Option<* mut StackFrame> = None;

        if end == 0 {
            Err(BdecodeError::UnexpectedEof(0))?
        }

        while start <= end {
            if stack.len() >= depth_limit as usize {
                Err(BdecodeError::DepthExceeded(depth_limit as usize))?
            }

            token_limit -= 1;
            if token_limit < 0 {
                Err(BdecodeError::LimitExceeded(DEFAULT_TOKEN_LIMIT as usize))?
            }

            // look for a new token
            let Some(t) = buffer.get(start) else {
                Err(BdecodeError::UnexpectedEof(start))?
            };

            // 检查当前是否在解析 dict 或 list 的过程中
            if let Some(stack_frame_ptr) = current_frame_ptr {
                let stack_frame = unsafe { *stack_frame_ptr };
                // 检查当前是否正要解析 dict
                if tokens[stack_frame.token() as usize].node_type() == BdecodeTokenType::Dict 
                    // 检查当前是否正要解析 dict 的 key
                    && stack_frame.state() == 0 
                    // 检查当前字符是否不为数字
                    && !t.is_ascii_digit()
                    // 检查当前字符是否不为 'e' ，如果是 'e' ，说明 dict 到了结尾
                    && *t != b'e'
                {
                    Err(BdecodeError::ExpectedDigit(start))?
                }
            }

            match t {
                b'd' => {
                    let frame = StackFrameBuilder::new()
                        .with_token(tokens.len() as u32)
                        .build();
                    stack.push(frame);
                    // 等 dict 解析完后再修正 next_item
                    tokens.push(BdecodeToken::new_dict(start as u32, 0));

                    start += 1;
                }
                b'l' => {
                    let frame = StackFrameBuilder::new()
                        .with_token(tokens.len() as u32)
                        .build();
                    stack.push(frame);
                    // 等 dict 解析完后再修正 next_item
                    tokens.push(BdecodeToken::new_list(start as u32, 0)); 

                    start += 1;
                }
                b'i' => {
                    let int_start = start;
                    start = check_integer(buffer.as_ref(), start + 1 as usize)?;
                    tokens.push(BdecodeToken::new_int(int_start as u32));

                    assert!(buffer[start] == b'e');

                    // skip 'e'
                    start += 1;
                }
                b'e' => {
                    if stack.is_empty() {
                        return Err(BdecodeError::UnexpectedEof(start));
                    }

                    // 检查当前是否在解析 dict 或 list 的过程中
                    if let Some(stack_frame) = stack.last() {
                        // 检查当前是否正要解析 dict
                        if tokens[stack_frame.token() as usize].node_type() == BdecodeTokenType::Dict 
                            // 检查当前是否正要解析 dict 的 value
                            && stack_frame.state() == 1
                        {
                            Err(BdecodeError::ExpectedValue(start))?
                        }
                    }

                    // 给 list 和 dict 的内部插入一个 end token，这样前一个的 item 的 next_item 就指向这个 end token.
                    tokens.push(BdecodeToken::new_end(start as u32));

                    // 计算当前 list 或 dict 的 next_item ----------

                    // top 是当前 list 或 dict 的入口 token 在 m_tokens 中的 index.
				    let top = stack.last().expect("stack is empty").token() as usize;
                    let next_item = tokens.len() - top;

                    if next_item > limits::MAX_NEXT_ITEM {
                        return Err(BdecodeError::LimitExceeded(limits::MAX_NEXT_ITEM));
                    }

                    // next_item 就是要跳过多少个 token.
                    tokens[top].set_next_item(next_item as u32);

                    stack.pop();
                    start += 1;
                }
                // parse 字符串 
                _ => {
                    if !t.is_ascii_digit() {
                        return Err(BdecodeError::ExpectedDigit(start));
                    }

                    let mut len = (t - b'0') as i64;
                    let str_start = start;
                    start += 1;

                    if start >= end {
                        return Err(BdecodeError::UnexpectedEof(start));
                    }

                    // 解析出后续字符串的 len 值，并返回 buffer 尚未解析的 start 位置
				    start = parse_uint(buffer.as_ref(), start, b':', &mut len)?;

                    if start == end {
                        return Err(BdecodeError::ExpectedColon(str_start, end));
                    }

                    // 截取 ':' 后的 buffer size
                    let buff_size = (end - start - 1) as i64;
                    if len > buff_size {
                        return Err(BdecodeError::UnexpectedEof(start));
                    }

                    // skip ':'
                    start += 1;
                    if start > end {
                        return Err(BdecodeError::UnexpectedEof(start));
                    }

                    // the bdecode_token only has 8 bits to keep the header size
                    // in. If it overflows, fail!
                    //
                    // eg: "10:abcdefghij2:kl"
                    //      ^^ ^
                    //      || |
                    //      || start(3), 也就是 bdecode_token() 构造函数中的 header_size 值
                    //      |bdecode_token.header 值
                    //      str_start(0)
                    //
                    // start - 1 = 2， 就是 "10" 的长度为 2
                    let header_size = start - str_start - 1;
                    if header_size > limits::MAX_HEADER_SIZE {
                        return Err(BdecodeError::LimitExceeded(limits::MAX_HEADER_SIZE));
                    }

                    tokens.push(BdecodeToken::new_str(str_start as u32, header_size as u8));
                    // 接上面的例子, 跳过整个字符串 "abcdefghij", 指向 "2:kl" 的 '2' 位置
				    start += len as usize;
                }
            }

            
            if let Some(stack_frame_prt) = current_frame_ptr {
                let stack_frame =  unsafe { stack_frame_prt.as_mut_unchecked() };
                // // 因为 vec 的 pop 只是将 sp - 1，并不会释放内存，所以下面的这段可以用后面的方法二代替。
                // // 方法一：
                // if let Some(now_frame_ptr) = stack.last() {
                //     let now_frame_ptr = now_frame_ptr as *const StackFrame;
                //     if now_frame_ptr >= stack_frame_prt {
                //         if tokens[stack_frame.token() as usize].node_type() == BdecodeTokenType::Dict {
                //             // 下一个我们解析的 Dict item 的 state 是一个相反的值，也就是从 key 切换到 value.
                //             let _state = stack_frame.state();
                //             stack_frame.set_state(!stack_frame.state());
                //         }
                //     }
                // } 

                // 方法二：
                // 注意：如果之前 stack 调用过 pop, 则下面写入时，会写到 stack 已经 pop 掉的位置，但是不会有读取，且不会报错。
                if tokens[stack_frame.token() as usize].node_type() == BdecodeTokenType::Dict {
                    // 下一个我们解析的 Dict item 的 state 是一个相反的值，也就是从 key 切换到 value.
                    let _state = stack_frame.state();
                    stack_frame.set_state(!stack_frame.state());
                }
            }

            // 保存处理完的 frame
            current_frame_ptr = stack.last_mut().map(|frame_ref| {
                frame_ref as *mut StackFrame 
            });

            // 如果当前栈为空，说明当前顶层节点也处理完了，则跳出循环. 也就是已经解析完整个 buffer 了。
            if stack.is_empty() {
                break;
            }
        } // end while

        // 推入一个虚拟 end token，用于结束解析
        tokens.push(BdecodeToken::new_end(start as u32));

        let (node_indexes, len) = gen_item_indexes(&tokens, 0);

        Ok(
            BdecodeNode {
                tokens: Arc::new(tokens),
                item_indexes: node_indexes,
                len,
                buffer,
                token_index: 0,
            }
        )
    }

    /// 获取当前节点的 token 的类型
    pub fn token_type(&self) -> BdecodeTokenType {
        if let Some(token) = self.tokens.get(self.token_index as usize) { 
            return token.node_type();
        } else {
            return BdecodeTokenType::None;
        }
    }

    /// 获取当前节点的整数值
    pub fn int_value(&self) -> BdecodeResult<i64> {
        assert!(self.token_type() == BdecodeTokenType::Int);

        let token_idx = self.token_index as usize;
        let t = &self.tokens[token_idx];
        let size = self.tokens[token_idx + 1].offset() - t.offset();

        // +1 is to skip the 'i'
        let start = t.offset() + 1;
        let mut val = 0;
        let mut negative = false;

        if  self.buffer[start as usize] == b'-' {
            negative = true;
        }

        let end = parse_uint(self.buffer.as_ref(), start as usize, b'e', &mut val)?;

        assert!(end < (start + size) as usize);

        if negative {
            Ok(-val)
        } else {
            Ok(val)
        }
    }

    /// 获取当前节点的字符串值
    pub fn string_value(&self) -> Cow<[u8]> {
        assert!(self.token_type() == BdecodeTokenType::Str);

        let token = &self.tokens[self.token_index as usize];
        let start = token.offset() as usize;
        let header_size = token.header_size() as usize + 1;
        let end = self.tokens[(self.token_index + 1) as usize].offset() as usize;

        let buf = &self.buffer[start + header_size ..end];
        // let rst = String::from_utf8_lossy(buf);
        let rst = Cow::Borrowed(buf);

        rst
    }

    /// 获取当前 list or dict 节点的长度
    pub fn len(&self) -> usize {
        assert!(matches!(self.token_type(), BdecodeTokenType::Dict | BdecodeTokenType::List));

        self.len
    }

    /// 获取 list 中指定索引的节点
    pub fn list_at(&self, index: usize) -> BdecodeNode {
        assert!(self.token_type() == BdecodeTokenType::List);

        if index >= self.len() {
            println!("index: {}, len: {}", index, self.len());
            panic!("index out of range");
        }
        
        let token_idx = self.item_indexes[index];
        let (node_indexes, len) = gen_item_indexes(&self.tokens, token_idx as usize);
        let node = BdecodeNode {
            buffer: self.buffer.clone(),
            tokens: self.tokens.clone(),
            token_index: token_idx,
            item_indexes: node_indexes,
            len,
        };

        node
    }

    pub fn list_string_value_at(&self, index: usize) -> Cow<[u8]> {
        let node = self.list_at(index);
        let val = node.string_value();

        let val_ptr = val.as_ref() as *const [u8];
        let val_ref = unsafe { &*val_ptr };

        Cow::Borrowed(val_ref)
    }

    pub fn list_int_value_at(&self, index: usize) -> BdecodeResult<i64> {
        self.list_at(index).int_value()
    }

    /// 获取 dict 中指定索引的节点对(key, value)
    pub fn dict_at(&self, index: usize) -> (BdecodeNode, BdecodeNode) {
        assert!(self.token_type() == BdecodeTokenType::Dict);

        if index >= self.len() {
            panic!("index out of range");
        }
        
        // get key node
        let key_token_idx = self.item_indexes[index];

        if key_token_idx as usize  >= self.tokens.len(){
            panic!("index out of range in tokens");
        }

        let key_node = BdecodeNode {
            buffer: self.buffer.clone(),
            tokens: self.tokens.clone(),
            token_index: key_token_idx,
            item_indexes: Default::default(),
            len: 0,
        };

        // get value node
        let key_token = &self.tokens[key_token_idx as usize];
        let val_token_idx = key_token_idx + key_token.next_item();
        let (node_indexes, len) = gen_item_indexes(&self.tokens, val_token_idx as usize);
        let val_node = BdecodeNode {
            buffer: self.buffer.clone(),
            tokens: self.tokens.clone(),
            token_index: val_token_idx,
            item_indexes: node_indexes,
            len,
        };

        (key_node, val_node)
    }

    /// 在 dict 中查找 key 对应的 value
    pub fn dict_find(&self, key: &[u8]) -> Option<BdecodeNode> {
        assert!(self.token_type() == BdecodeTokenType::Dict);

        for token_index in self.item_indexes.as_ref() {
            let token = &self.tokens[*token_index as usize];
            assert!(token.node_type() == BdecodeTokenType::Str);
            let next_offset = self.tokens[(token_index + 1) as usize].offset() as usize;
            let start = (token.offset() + token.header_size() as u32 + 1) as usize;
            
            if &self.buffer[start..next_offset] == key {
                let val_token_idx = *token_index + token.next_item();
                let (node_indexes, len) = gen_item_indexes(&self.tokens, val_token_idx as usize);
                return Some(BdecodeNode {
                    buffer: self.buffer.clone(),
                    tokens: self.tokens.clone(),
                    token_index: val_token_idx,
                    item_indexes: node_indexes,
                    len,
                });
            }
        }

        None
    }

    pub fn dict_find_string_value(&self, key: &[u8]) -> Option<Cow<[u8]>> {
        let node = self.dict_find(key);
        
        if let Some(node) = node {
            let val = node.string_value();
            let val_ptr = val.as_ref() as *const [u8];
            let val_ref = unsafe { &*val_ptr };

            let rst = Cow::Borrowed(val_ref);

            return Some(rst);
        }
        
        None
    }

    pub fn dict_find_int_value(&self, key: &[u8]) -> Option<i64> {
        let node = self.dict_find(key);
        
        if let Some(node) = node {
            return node.int_value().ok()
        }

        None
    }

    pub fn dict_find_list(&self, key: &[u8]) -> Option<Vec<BdecodeNode>> {
        let node = self.dict_find(key);
        
        if let Some(node) = node {
            return if node.token_type() == BdecodeTokenType::List {
                let mut nodes = vec![];
                for i in 0..node.len() {
                    let node = node.list_at(i);
                    nodes.push(node);
                }

                Some(nodes)
            } else {
                None
            }
        }

        None
    }

    pub fn dict_find_dict(&self, key: &[u8]) -> Option<HashMap<Cow<[u8]>, BdecodeNode>> {
        let Some(node) = self.dict_find(key) else {
            return None;
        };

        let mut node_map = HashMap::new();
        for i in 0..node.len() {
            let (key, value) = node.dict_at(i);

            let key_str = key.string_value();
            let key_ptr = key_str.as_ref() as *const [u8];
            let key_ref = unsafe { &*key_ptr };

            let key = Cow::Borrowed(key_ref);

            node_map.insert(key, value);
        }

        Some(node_map)
    }
}

/// 为一个 Bdecode 节点生成它的子节点的索引列表，以及长度。
fn gen_item_indexes(tokens: &[BdecodeToken], start_token_idx: usize) -> (Arc<Vec<u32>>, usize) {
    use BdecodeTokenType::*;

    assert!(start_token_idx < tokens.len());

    if tokens.len() < 2 {
        return (Default::default(), 0)
    }
    
    let mut node_indexes = vec![];
    let mut count = 0;
    
    let mut begin = 1 + start_token_idx;
    match tokens[start_token_idx].node_type() {
        Dict => {
            while tokens[begin].node_type() != End {
                if count % 2 == 0 {
                    node_indexes.push(begin as u32);
                }
                count += 1;

                begin += tokens[begin].next_item() as usize;
            }
            count /= 2;
        }
        List => {
            while tokens[begin].node_type() != End {
                node_indexes.push(begin as u32);
                begin += tokens[begin].next_item() as usize;
                count += 1;
            }
        }
        _ => (),
    }

    (Arc::new(node_indexes), count)
}

impl core::fmt::Debug for BdecodeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BdecodeNode")
            .field("token_idx", &self.token_index)
            .field("item_indexes", &self.item_indexes)
            .field("tokens", &self.tokens)
            .field("buffer", &bytes::Bytes::copy_from_slice(&self.buffer))
            .finish()
    }
}

impl core::fmt::Display for BdecodeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let mut tokens = self.tokens.clone();
        f.debug_struct("BdecodeNode")
            .field("token_idx", &self.token_index)
            .field("tokens", &self.tokens)
            .field("buffer", &bytes::Bytes::copy_from_slice(&self.buffer))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_at() {
        // [19, "ab", {"k1": "v1", "k2": [1, 2]} ]
        let buffer: Arc<Vec<u8>> = Arc::new("l i19e 2:ab d 2:k1 2:v1 2:k2 l i1e i2e e e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(19, node.list_at(0).int_value().unwrap());
        assert_eq!(19, node.list_int_value_at(0).unwrap());
        assert_eq!(b"ab", node.list_at(1).string_value().as_ref());

        let node_2 = node.list_at(2);
        assert_eq!(BdecodeTokenType::Dict, node_2.token_type());
        assert_eq!(3, node_2.token_index);
        assert_eq!(&vec![4, 6], node_2.item_indexes.as_ref());
        assert_eq!(2, node.list_at(2).len());

        assert_eq!(b"ab", node.list_string_value_at(1).as_ref());
    }

    #[test]
    fn test_dict_at() {
        // [19, "ab", {"k1": "v1", "k2": [1, 2]} ]
        let buffer: Arc<Vec<u8>> = Arc::new("l i19e 2:ab d 2:k1 2:v1 2:k2 l i1e i2e e e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(3, node.len());

        let node_2 = node.list_at(2);
        assert_eq!(2, node_2.len());

        let (key, val) = node_2.dict_at(0);
        assert_eq!(b"k1", key.string_value().as_ref());
        assert_eq!(b"v1", val.string_value().as_ref());

        let (key, val) = node_2.dict_at(1);
        assert_eq!(b"k2", key.string_value().as_ref());
        assert_eq!(7, val.token_index);
        assert_eq!(2, val.len());
        assert_eq!(&vec![8, 9], val.item_indexes.as_ref());
    }

    #[test]
    fn test_dict_find() {
        // {"k1": "v1", "k2": [1, 2], "k03": 3, "k4": {"k5": 5, "k6": 6}}
        let buffer: Arc<Vec<u8>> = Arc::new("d 2:k1 2:v1 2:k2 l i1e i2e e 3:k03 i3e 2:k4 d 2:k5 i5e 2:k6 i6e e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(4, node.len());

        let val_1 = node.dict_find(b"k1").unwrap();
        assert_eq!(b"v1", val_1.string_value().as_ref());

        let val_3 = node.dict_find(b"k03").unwrap();
        assert_eq!(3, val_3.int_value().unwrap());

        let val_2 = node.dict_find(b"k2").unwrap();
        assert_eq!(BdecodeTokenType::List, val_2.token_type());
        assert_eq!(4, val_2.token_index);
        assert_eq!(2, val_2.len());
        assert_eq!(1, val_2.list_at(0).int_value().unwrap());
        assert_eq!(2, val_2.list_at(1).int_value().unwrap());

        let v1 = node.dict_find_string_value(b"k1");
        assert_eq!(b"v1", v1.unwrap().as_ref());

        let v03 = node.dict_find_int_value(b"k03");
        assert_eq!(3, v03.unwrap());

        let v2 = node.dict_find_list(b"k2").unwrap();
        assert_eq!(5, v2[0].token_index);
        assert_eq!(6, v2[1].token_index);

        let v4 = node.dict_find_dict(b"k4").unwrap();
        let v5 = v4.get(b"k5".as_ref()).unwrap();
        assert_eq!(5, v5.int_value().unwrap());
        let v6 = v4.get(b"k6".as_ref()).unwrap();
        assert_eq!(6, v6.int_value().unwrap());
    }

    #[test]
    #[should_panic(expected = "index out of range")]
    fn test_panic_list_at() {
        // [19, "ab", "cd", "ef"]
        let buffer: Arc<Vec<u8>> = Arc::new("l i19e 2:ab 2:cd 2:ef e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        let _ = node.list_at(4);
    }

    #[test]
    fn test_gen_item_indexes() {
        // 2:v1
        let v_1 = BdecodeToken::new_str(0, 1);
        let e_x = BdecodeToken::new_end(1);
        let tokens = vec![v_1, e_x];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![]), 0));

        // {"k1": "v1", "k2": [1, 2], "k3": 3}
        // str  | pos   | seq
        // --------------------
        // d_1  | 0     | 0
        // 2:k1 | 1     | 1
        // 2:v1 | 5     | 2
        // 2:k2 | 9     | 3
        // l_2  | 13    | 4
        // i1e  | 14    | 5
        // i2e  | 17    | 6
        // e_2  | 20    | 7
        // 2:k3 | 21    | 8
        // i3e  | 25    | 9
        // e_1  | 28    | 10
        // e_x  | 29    | 11
        let d_1 = BdecodeToken::new_dict(0, 11);
        let k_1 = BdecodeToken::new_str(1, 1);
        let v_1 = BdecodeToken::new_str(5, 1);
        let k_2 = BdecodeToken::new_str(9, 1);
        let l_2 = BdecodeToken::new_list(13, 4);
        let i_1 = BdecodeToken::new_int(14);
        let i_2 = BdecodeToken::new_int(17);
        let e_2 = BdecodeToken::new_end(20);
        let k_3 = BdecodeToken::new_str(21, 1);
        let i_3 = BdecodeToken::new_int(25);
        let e_1 = BdecodeToken::new_end(28);
        let e_x = BdecodeToken::new_end(29);
        let tokens = vec![ d_1, k_1, v_1, k_2, l_2, i_1, i_2, e_2, k_3, i_3, e_1, e_x ];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![1, 3, 8]), 3));

        // [1, [2], {"k4": 4}]
        // str  | pos   | seq
        // --------------------
        // l_1  | 0     | 0
        // i1e  | 1     | 1
        // l_2  | 4     | 2
        // i2e  | 5     | 3
        // e_2  | 8     | 4
        // d_3  | 9     | 5
        // 2:k4 | 10    | 6
        // i4e  | 14    | 7
        // e_3  | 17    | 8
        // e_x  | 18    | 9
        let l_1 = BdecodeToken::new_list(0, 9);
        let i_1 = BdecodeToken::new_int(1);
        let l_2 = BdecodeToken::new_list(4, 3);
        let i_2 = BdecodeToken::new_int(5);
        let e_2 = BdecodeToken::new_end(8);
        let d_3 = BdecodeToken::new_dict(9, 4);
        let k_4 = BdecodeToken::new_str(10, 1);
        let i_4 = BdecodeToken::new_int(14);
        let e_3 = BdecodeToken::new_end(17);
        let e_x = BdecodeToken::new_end(18);
        let tokens = vec![l_1, i_1, l_2, i_2, e_2, d_3, k_4, i_4, e_3, e_x];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![1, 2, 5]), 3));
    }

    #[test]
    fn test_string_value() {
        let buffer: Arc<Vec<u8>> = Arc::new("11:k1000000012".into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(node.string_value().as_ref(), b"k1000000012");
    }

    #[test]
    fn test_int_value() {
        let buffer: Arc<Vec<u8>> = Arc::new("i19e".into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(node.int_value().unwrap(), 19);
    }

    #[test]
    fn test_token_type() {
        let buffer: Arc<Vec<u8>> = Arc::new("2:k1".into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(BdecodeTokenType::Str, node.token_type());
    }

    #[test]
    fn test_new_bdecode_node() {
        // "k1"
        let buffer: Arc<Vec<u8>> = Arc::new("2:k1".into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // k1 root_end 总共 2 个 token.
        assert_eq!(2, node.tokens.len());

        // 19
        let buffer: Arc<Vec<u8>> = Arc::new("i19e".into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(2, node.tokens.len());

        // [19, "ab"]
        let buffer: Arc<Vec<u8>> = Arc::new("l i19e 2:ab e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, 19, "ab", list_end, root_end 总共 5 个 token.
        assert_eq!(5, node.tokens.len());
        assert_eq!(2, node.len());

        // {"a": "b", "cd": "foo", "baro": 9}
        let buffer = Arc::new("d 1:a 1:b 2:cd 3:foo 4:baro i9e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(node.tokens.len(), 9);
        assert_eq!(3, node.len());

        // {"k1": "v1", "k2": {"k3": "v3", "k4": 9}, k5: [7, 8], k6: "v6"}
        let buffer = Arc::new("d 2:k1 2:v1 2:k2 d 2:k3 2:v3 2:k4 i9e e 2:k5 l i7e i8e e 2:k6 2:v6 e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(node.tokens.len(), 19);
        assert_eq!(4, node.len());

        // {"k111111111": "v1", "k2": {"k3": 9}}
        let buffer: Arc<Vec<u8>> = Arc::new("d 10:k111111111 2:v1 2:k2 d 2:k3 i9e e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, k111111111, v1, k2, dict_2, k3, i9e, inner_end, root_end 总共 9 个 token.
        assert_eq!(10, node.tokens.len());
        assert_eq!(2, node.len());

        // {"k1": [9], "k2": 2}
        // str | state_处理前 - stack_frame(state_处理后) | pos
        // --------------------
        // d_1     X - d_1(0)  0
        // 2:k1    0 - d_1(1)  1
        // l_2     1 - d_1(0)  5
        // i9e     X - l_2     6
        // e_2     X - l_2     9
        // 2:k2    0 - d_1(1)  10
        // i2e     1 - d_1(0)  14
        // e_1     0 - d_1(1)  17
        // e_x     1 - d_1(0)  18
        let buffer: Arc<Vec<u8>> = Arc::new("d 2:k1 l i9e e 2:k2 i2e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(9, node.tokens.len());
        assert_eq!(2, node.len());

        // {"k1": {"k2": 9}, "k3": 3}
        // str | state_处理前 - stack_frame(state_处理后) | pos
        // --------------------------
        // d_1     X - d_1(0)   0
        // 2:k1    0 - d_1(1)   1
        // d_2     1 - d_1(0)   5
        // 2:k2    0 - d_2(1)   6
        // i9e     1 - d_2(0)   10
        // e_2     0 - d_2(1)*  13
        // 2:k3    0 - d_1(1)   14
        // i3e     1 - d_1(0)   18
        // e_1     0 - d_1(1)   21
        // e_x     1 - d_1(0)   22
        let buffer: Arc<Vec<u8>> = Arc::new("d 2:k1 d 2:k2 i9e e 2:k3 i3e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(10, node.tokens.len());
        assert_eq!(2, node.len());

        // {"k1": {"k2": {"k3": [9]}}, "k4": "4"}
        // str | state_处理前 - stack_frame(state_处理后) | pos
        // --------------------------------------
        // d_1     X - d_1(0)   0
        // 2:k1    0 - d_1(1)   1
        // d_2     1 - d_1(0)   5
        // 2:k2    0 - d_2(1)   6
        // d_3     1 - d_2(0)   10
        // 2:k3    0 - d_3(1)   11
        // l_4     1 - d_3(0)   15
        // i9e     X - l_4      16
        // e_4     X - l_4      19
        // e_3     1 - d_3(0)   20
        // e_2     0 - d_2(1)*  21
        // 2:k4    0 - d_1(1)   22
        // 1:4     1 - d_1(0)   26
        // e_1     1 - d_1(0)   29
        // e_x     1 - d_1(0)   30
        let buffer: Arc<Vec<u8>> = Arc::new("d 2:k1 d 2:k2 d 2:k3 l i9e e e e 2:k4 1:4 e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        assert_eq!(15, node.tokens.len());
        assert_eq!(2, node.len());
    }
}

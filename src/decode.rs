mod dict;
mod end;
mod int;
mod list;
mod node;
mod stack_frame;
mod str;
mod utils;
mod macros;
mod commons;
mod token;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use commons::limits::{self, BUFFER_MAX_OFFSET, DEFAULT_DEPTH_LIMIT, DEFAULT_TOKEN_LIMIT};
use stack_frame::{StackFrame, StackFrameBuilder};
use token::{BdecodeToken, BdecodeTokenType};
use utils::{check_integer, gen_item_indexes, parse_uint};

pub use {dict::*, end::*, int::*, list::*, node::*, str::*};

use crate::{BdecodeError, BdecodeResult};

#[derive(PartialEq, Eq)]
pub enum Style {
    Compact,
    Pretty(usize),
}

/// 用于存放解析后的数据
#[derive(Clone)]
pub enum BdecodeNode {
    Dict(Dict),
    List(List),
    Str(Str),
    Int(Int),
    End(End),
}

impl BdecodeNode {
    pub fn new(
        token_idx: u32,
        tokens: Arc<Vec<BdecodeToken>>,
        buffer: Arc<Vec<u8>>,
    ) -> BdecodeNode {
        let token = &tokens[token_idx as usize];
        let node = match token.node_type() {
            BdecodeTokenType::Str => {
                let v = Str::new(buffer, tokens, token_idx);
                BdecodeNode::Str(v)
            }
            BdecodeTokenType::Int => {
                let v = Int::new(buffer, tokens, token_idx);
                BdecodeNode::Int(v)
            }
            BdecodeTokenType::List => {
                let (item_indexes, len) = gen_item_indexes(&tokens, token_idx as usize);
                let v = List::new(buffer, tokens, token_idx, item_indexes, len);

                BdecodeNode::List(v)
            }
            BdecodeTokenType::Dict => {
                let (item_indexes, len) = gen_item_indexes(&tokens, token_idx as usize);
                let v = Dict::new(buffer, tokens, token_idx, item_indexes, len);

                BdecodeNode::Dict(v)
            }
            BdecodeTokenType::End => {
                let v = End::new(buffer, tokens, token_idx);
                BdecodeNode::End(v)
            }
        };
        node
    }

    pub fn as_int(&self) -> BdecodeResult<i64> {
        let BdecodeNode::Int(inner_node) = self else {
            panic!("not a Int node")
        };

        inner_node.value()
    }

    pub fn as_str(&self) -> Cow<[u8]> {
        let BdecodeNode::Str(inner_node) = self else {
            panic!("not a Str node")
        };

        inner_node.value()
    }

    pub fn len(&self) -> usize {
        use BdecodeNode::*;

        match self {
            List(inner_node) => inner_node.len(),
            Dict(inner_node) => inner_node.len(),
            _ => panic!("not a List or Dict node"),
        }
    }

    pub fn list_item(&self, index: usize) -> BdecodeNode {
        let BdecodeNode::List(inner_node) = self else {
            panic!("not a List node")
        };

        inner_node.item(index)
    }

    pub fn list_item_as_int(&self, index: usize) -> BdecodeResult<i64> {
        let BdecodeNode::List(inner_node) = self else {
            panic!("not a List node")
        };

        inner_node.as_int(index)
    }

    pub fn list_item_as_str(&self, index: usize) -> Cow<[u8]> {
        let BdecodeNode::List(inner_node) = self else {
            panic!("not a List node")
        };

        inner_node.as_str(index)
    }

    pub fn dict_item(&self, index: usize) -> (BdecodeNode, BdecodeNode) {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.item(index)
    }

    pub fn dict_find(&self, key: &[u8]) -> Option<BdecodeNode> {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.find(key)
    }

    pub fn dict_find_as_str(&self, key: &[u8]) -> Option<Cow<[u8]>> {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.find_as_str(key)
    }

    pub fn dict_find_as_int(&self, key: &[u8]) -> Option<i64> {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.find_as_int(key)
    }

    pub fn dict_find_as_list(&self, key: &[u8]) -> Option<Vec<BdecodeNode>> {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.find_as_list(key)
    }

    pub fn dict_find_as_dict(&self, key: &[u8]) -> Option<HashMap<Cow<[u8]>, BdecodeNode>> {
        let BdecodeNode::Dict(inner_node) = self else {
            panic!("not a Dict node")
        };

        inner_node.find_as_dict(key)
    }

    pub fn parse(
        buffer: Vec<u8>,
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

        Ok(BdecodeNode::new(0, Arc::new(tokens), Arc::new(buffer)))
    }

    pub fn parse_buffer(buffer: Vec<u8>) -> BdecodeResult<Self> {
        Self::parse(buffer, None, None)
    }

    pub fn to_json_with_style(&self, style: Style) -> String {
        match self {
            BdecodeNode::Dict(inner_node) => inner_node.to_json_with_style(style),
            BdecodeNode::List(inner_node) => inner_node.to_json_with_style(style),
            BdecodeNode::Str(inner_node) => inner_node.to_json_with_style(style),
            BdecodeNode::Int(inner_node) => inner_node.to_json_with_style(style),
            BdecodeNode::End(inner_node) => inner_node.to_json_with_style(style),
        }
    }

    pub fn to_json(&self) -> String {
        self.to_json_with_style(Style::Compact)
    }

    pub fn to_json_pretty(&self) -> String {
        self.to_json_with_style(Style::Pretty(0))
    }
}

impl core::fmt::Debug for BdecodeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BdecodeNode::Dict(inner_node) => {
                f.debug_struct("Dict")
                    .field("token_idx", &inner_node.token_index)
                    .field("item_indexes", &inner_node.item_indexes)
                    .field("len", &inner_node.len())
                    .field("tokens", &inner_node.tokens)
                    .field("buffer", &bytes::Bytes::copy_from_slice(&inner_node.buffer))
                    .finish()
            }
            BdecodeNode::List(inner_node) => {
                f.debug_struct("List")
                    .field("token_idx", &inner_node.token_index)
                    .field("item_indexes", &inner_node.item_indexes)
                    .field("len", &inner_node.len())
                    .field("tokens", &inner_node.tokens)
                    .field("buffer", &bytes::Bytes::copy_from_slice(&inner_node.buffer))
                    .finish()
            }
            BdecodeNode::Str(inner_node) => {
                f.debug_struct("Str")
                    .field("token_idx", &inner_node.token_index)
                    .field("tokens", &inner_node.tokens)
                    .field("buffer", &bytes::Bytes::copy_from_slice(&inner_node.buffer))
                    .finish()
            }
            BdecodeNode::Int(inner_node) => {
                f.debug_struct("Int")
                    .field("token_idx", &inner_node.token_index)
                    .field("tokens", &inner_node.tokens)
                    .field("buffer", &bytes::Bytes::copy_from_slice(&inner_node.buffer))
                    .finish()
            }
            BdecodeNode::End(inner_node) => {
                f.debug_struct("End")
                    .field("token_idx", &inner_node.token_index)
                    .field("tokens", &inner_node.tokens)
                    .field("buffer", &bytes::Bytes::copy_from_slice(&inner_node.buffer))
                    .finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        // {"\x04b": "v\x02", "k2": {"k3": "v3", "k4": 9}, "k5": [7, {"b1": "bb"}], "k6": "v6"}
        let buffer = b"d 2:\x04b 2:v\x02 2:k2 d 2:k3 2:v3 2:k4 i9e e 2:k5 l i7e d 2:b1 2:bb e e 2:k6 2:v6 e"
            .into_iter()
            .filter(|v| {
                **v != b' '
            })
            .cloned()
            .collect::<Vec<_>>();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        println!("{}", &node.to_json_pretty());
        println!("{}", &node.to_json());
    }

    #[test]
    fn test_new_bdecode_node() {
        // "k1"
        let buffer = "2:k1";
        let node = BdecodeNode::parse_buffer(buffer.into()).unwrap();
        let BdecodeNode::Str(node) = node else {
            panic!("not a Str node");
        };
        assert_eq!(2, node.tokens.len());

        // 19
        let buffer = "i19e";
        let node = BdecodeNode::parse_buffer(buffer.into()).unwrap();
        let BdecodeNode::Int(node) = node else {
            panic!("not a Int node");
        };
        assert_eq!(2, node.tokens.len());

        // [19, "ab"]
        let buffer = "l i19e 2:ab e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::List(node) = node else {
            panic!("not a List node");
        };
        assert_eq!(5, node.tokens.len());
        assert_eq!(2, node.len());

        // {"a": "b", "cd": "foo", "baro": 9}
        let buffer = "d 1:a 1:b 2:cd 3:foo 4:baro i9e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
        assert_eq!(node.tokens.len(), 9);
        assert_eq!(3, node.len());

        // {"k1": "v1", "k2": {"k3": "v3", "k4": 9}, k5: [7, 8], k6: "v6"}
        let buffer = "d 2:k1 2:v1 2:k2 d 2:k3 2:v3 2:k4 i9e e 2:k5 l i7e i8e e 2:k6 2:v6 e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
        assert_eq!(node.tokens.len(), 19);
        assert_eq!(4, node.len());

        // {"k111111111": "v1", "k2": {"k3": 9}}
        let buffer = "d 10:k111111111 2:v1 2:k2 d 2:k3 i9e e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
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
        let buffer = "d 2:k1 l i9e e 2:k2 i2e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
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
        let buffer = "d 2:k1 d 2:k2 i9e e 2:k3 i3e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
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
        let buffer = "d 2:k1 d 2:k2 d 2:k3 l i9e e e e 2:k4 1:4 e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let BdecodeNode::Dict(node) = node else {
            panic!("not a Dict node");
        };
        assert_eq!(15, node.tokens.len());
        assert_eq!(2, node.len());
    }

    #[test]
    fn test_list_at() {
        // [19, "ab", {"k1": "v1", "k2": [1, 2]} ]
        let buffer = "l i19e 2:ab d 2:k1 2:v1 2:k2 l i1e i2e e e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert_eq!(19, node.list_item(0).as_int().unwrap());
        assert_eq!(19, node.list_item_as_int(0).unwrap());
        assert_eq!(b"ab", node.list_item(1).as_str().as_ref());

        let node_2 = node.list_item(2);
        assert!(matches!(node_2, BdecodeNode::Dict(_)));
        assert_eq!(2, node.list_item(2).len());

        assert_eq!(b"ab", node.list_item_as_str(1).as_ref());
    }

    #[test]
    fn test_dict_item() {
        // [19, "ab", {"k1": "v1", "k2": [1, 2]} ]
        let buffer = "l i19e 2:ab d 2:k1 2:v1 2:k2 l i1e i2e e e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert_eq!(3, node.len());

        let node_2 = node.list_item(2);
        assert_eq!(2, node_2.len());

        let (key, val) = node_2.dict_item(0);
        assert_eq!(b"k1", key.as_str().as_ref());
        assert_eq!(b"v1", val.as_str().as_ref());

        let (key, val) = node_2.dict_item(1);
        assert_eq!(b"k2", key.as_str().as_ref());
        let BdecodeNode::List(val) = val else {
            panic!("not a List node");
        };
        assert_eq!(7, val.token_index());
        assert_eq!(2, val.len());
        assert_eq!(&vec![8, 9], val.item_indexes.as_ref());
    }

    #[test]
    fn test_dict_find() {
        // {"k1": "v1", "k2": [1, 2], "k03": 3, "k4": {"k5": 5, "k6": 6}}
        let buffer = "d 2:k1 2:v1 2:k2 l i1e i2e e 3:k03 i3e 2:k4 d 2:k5 i5e 2:k6 i6e e e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert_eq!(4, node.len());

        let val_1 = node.dict_find(b"k1").unwrap();
        assert_eq!(b"v1", val_1.as_str().as_ref());

        let val_3 = node.dict_find(b"k03").unwrap();
        assert_eq!(3, val_3.as_int().unwrap());

        let val_2 = node.dict_find(b"k2").unwrap();
        assert!(matches!(val_2, BdecodeNode::List(_)));
        let BdecodeNode::List(val_2) = val_2 else {
            panic!("not a List node");
        };
        assert_eq!(4, val_2.token_index());
        assert_eq!(2, val_2.len());
        assert_eq!(1, val_2.item(0).as_int().unwrap());
        assert_eq!(2, val_2.item(1).as_int().unwrap());

        let v1 = node.dict_find_as_str(b"k1");
        assert_eq!(b"v1", v1.unwrap().as_ref());

        let v03 = node.dict_find_as_int(b"k03");
        assert_eq!(3, v03.unwrap());

        let v2 = node.dict_find_as_list(b"k2").unwrap();
        let BdecodeNode::Int(v2_0) = &v2[0] else {
            panic!("not a Int node");
        };
        assert_eq!(5, v2_0.token_index());
        let BdecodeNode::Int(v2_1) = &v2[1] else {
            panic!("not a Int node");
        };
        assert_eq!(6, v2_1.token_index());

        let v4 = node.dict_find_as_dict(b"k4").unwrap();
        let v5 = v4.get(b"k5".as_ref()).unwrap();
        assert_eq!(5, v5.as_int().unwrap());
        let v6 = v4.get(b"k6".as_ref()).unwrap();
        assert_eq!(6, v6.as_int().unwrap());
    }

    #[test]
    #[should_panic(expected = "index out of range")]
    fn test_panic_list_at() {
        // [19, "ab", "cd", "ef"]
        let buffer = "l i19e 2:ab 2:cd 2:ef e".replace(" ", "").into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        let _ = node.list_item(4);
    }

    #[test]
    fn test_string_value() {
        let buffer = "11:k1000000012".into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert_eq!(node.as_str().as_ref(), b"k1000000012");
    }

    #[test]
    fn test_int_value() {
        let buffer = "i19e".into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert_eq!(node.as_int().unwrap(), 19);
    }

    #[test]
    fn test_node_type() {
        let buffer = "2:k1".into();
        let node = BdecodeNode::parse_buffer(buffer).unwrap();
        assert!(matches!(node, BdecodeNode::Str(_)))
    }
}
mod utils;

use std::sync::Arc;

use bitfields::bitfield;

use crate::{
    limits::{self, BUFFER_MAX_OFFSET, DEFAULT_DEPTH_LIMIT, DEFAULT_TOKEN_LIMIT},
    utils::{check_integer, parse_uint},
    BdecodeError, BdecodeResult, BdecodeToken, BdecodeTokenType,
};

#[bitfield(u32)]
#[derive(Clone, Copy)]
struct StackFrame {
    #[bits(31)]
    token: u32,

    #[bits(1)]
    state: u8,
}

// #[derive(Debug)]
pub struct BdecodeNode {
    /// 当前节点在 tokens 中的索引
    /// 0 - root 节点值; -1 - 未初始化  
    pub token_idx: u32,

    /// 存放 list 和 map 中 item 的 begin_index 和 len
    pub item_begin_len: Option<(u32, u32)>,

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
            let t = buffer[start];

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
                    && t != b'e'
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
                    tokens.push(BdecodeToken::new_dict(start as u32));

                    start += 1;
                }
                b'l' => {
                    let frame = StackFrameBuilder::new()
                        .with_token(tokens.len() as u32)
                        .build();
                    stack.push(frame);
                    tokens.push(BdecodeToken::new_list(start as u32));

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
                if tokens[stack_frame.token() as usize].node_type() == BdecodeTokenType::Dict {
                    // 下一个我们解析的 Dict item 的 state 是一个相反的值，也就是从 key 切换到 value.
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

        let item_begin_len = if tokens.len() >= 4 
            && matches!(tokens[0].node_type(), BdecodeTokenType::Dict | BdecodeTokenType::List) 
        {
            Some((1, (tokens.len() - 3) as u32))
        } else {
            None
        };

        Ok(
            BdecodeNode {
                tokens: Arc::new(tokens),
                item_begin_len,
                buffer,
                token_idx: 0,
            }
        )
    }

    pub fn token_type(&self) -> BdecodeTokenType {
        if let Some(token) = self.tokens.get(self.token_idx as usize) { 
            return token.node_type();
        } else {
            return BdecodeTokenType::None;
        }
    }

    pub fn int_value(&self) -> BdecodeResult<i64> {
        assert!(self.token_type() == BdecodeTokenType::Int);

        let token_idx = self.token_idx as usize;
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
}

impl core::fmt::Debug for BdecodeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BdecodeNode")
            .field("token_idx", &self.token_idx)
            .field("item_begin_len", &self.item_begin_len)
            .field("tokens", &self.tokens)
            .field("buffer", &bytes::Bytes::copy_from_slice(&self.buffer))
            .finish()
    }
}

impl core::fmt::Display for BdecodeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let mut tokens = self.tokens.clone();
        f.debug_struct("BdecodeNode")
            .field("token_idx", &self.token_idx)
            .field("item_begin_len", &self.item_begin_len)
            .field("tokens", &self.tokens)
            .field("buffer", &bytes::Bytes::copy_from_slice(&self.buffer))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // 19 root_end 总共 2 个 token.
        assert_eq!(2, node.tokens.len());

        // [19, "ab"]
        let buffer: Arc<Vec<u8>> = Arc::new("l i19e 2:ab e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, 19, "ab", list_end, root_end 总共 5 个 token.
        assert_eq!(5, node.tokens.len());

        // {"a": "b", "cd": "foo", "baro": 9}
        let buffer = Arc::new("d 1:a 1:b 2:cd 3:foo 4:baro i9e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, a, b, cd, foo, baro, 9, inner_end, root_end 总共 9 个 token.
        assert_eq!(node.tokens.len(), 9);

        // {"k1": "v1", "k2": {"k3": "v3", "k4": 9}, k5: [7, 8], k6: "v6"}
        let buffer = Arc::new("d 2:k1 2:v1 2:k2 d 2:k3 2:v3 2:k4 i9e e 2:k5 l i7e i8e e 2:k6 2:v6 e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, a, b, cd, foo, baro, 9, inner_end, root_end 总共 9 个 token.
        assert_eq!(node.tokens.len(), 19);

        // {"k1": "v1", "k2": {"k3": 9}}
        let buffer: Arc<Vec<u8>> = Arc::new("d 10:k111111111 2:v1 2:k2 d 2:k3 i9e e e".replace(" ", "").into());
        let node = BdecodeNode::with_buffer(buffer).unwrap();
        // root, k1, v1, k2, dict_2, k3, i9e, inner_end, root_end 总共 9 个 token.
        assert_eq!(10, node.tokens.len());
    }
}

use std::sync::Arc;

use bitfields::bitfield;

use crate::{
    limits::{self, BUFFER_MAX_OFFSET, DEFAULT_DEPTH_LIMIT, DEFAULT_TOKEN_LIMIT},
    utils::check_integer,
    BdecodeError, BdecodeResult, BdecodeToken, BdecodeTokenType,
};

#[bitfield(u32)]
struct StackFrame {
    #[bits(31)]
    token: u32,

    #[bits(1)]
    state: u8,
}
pub struct BdecodeNode {
    /// 解析后的 token 集合
    pub tokens: Arc<Vec<BdecodeToken>>,

    /// 存放解析前字符串的 buffer
    pub buffer: Arc<Vec<u8>>,

    /// 当前节点在 tokens 中的索引
    /// 0 - root 节点值; -1 - 未初始化  
    pub token_idx: usize,

    /// 存放 list 和 map 中 item 的 item_index 和 token_index 的集合，用于快速获取 item 值
    pub item_token_indexes: Arc<Vec<usize>>,
}

impl BdecodeNode {
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
            if let Some(stack_frame) = stack.last() {
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
                _ => {
                    // parse 字符串 
                    todo!()
                }
            }
        }

        todo!()
    }
}

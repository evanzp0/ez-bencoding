use bitfields::bitfield;

/// token 类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BdecodeTokenType {
    /// 不存在的节点类型(未初始化或默认构造的节点)
    None = 0,
    /// 字典
    Dict,
    /// 列表
    List,
    /// 字符串
    Str,
    /// 整型
    Int,
    /// 结束(一个虚拟节点)
    End,
}

impl BdecodeTokenType {
    ///Creates a new bitfield instance from the given bits.
    pub const fn from_bits(bits: u8) -> Self {
        match bits {
            1 => Self::Dict,
            2 => Self::List,
            3 => Self::Str,
            4 => Self::Int,
            5 => Self::End,
            _ => Self::None,
        }
    }

    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

/// Bdecode 分词
/// 用来结构化描述 buffer 中 bencoding 编码的字符串
#[bitfield(u64)]
pub struct BdecodeToken {
    /// 当前节点在 bdecoded buffer 中对应的偏移位置
    #[bits(29)]
    offset: u32,

    /// 当前节点类型
    #[bits(3)]
    node_type: BdecodeTokenType,

    /// 下一个节点在 tokens vector 中相对于当前节点的偏移索引值
    #[bits(29)]
    next_item: u32,

    /// 字符串在 bdecoded buffer 中, ':' 前的代表整数的字符串长度值
    /// 
    /// 例如：
    /// "10:abcdefghij" 中的 header 值是 '10', 所以 header_size 为 2
    #[bits(3)]
    header_size: u8,
}

impl BdecodeToken {
    pub fn new_all(offset: u32, node_type: BdecodeTokenType, next_item: u32, head_size: u8) -> Self {
        BdecodeTokenBuilder::new()
            .with_offset(offset)
            .with_node_type(node_type)
            .with_next_item(next_item)
            .with_header_size(head_size)
            .build()
    }

    pub fn new_dict(offset: u32, next_item: u32) -> Self {
        Self::new_all(offset, BdecodeTokenType::Dict, next_item, 0)
    }

    pub fn new_list(offset: u32, next_item: u32) -> Self {
        Self::new_all(offset, BdecodeTokenType::List, next_item, 0)
    }

    pub fn new_int(offset: u32) -> Self {
        let next_item = 1;
        Self::new_all(offset, BdecodeTokenType::Int, next_item, 0)
    }

    pub fn new_end(offset: u32) -> Self {
        let next_item = 1;
        Self::new_all(offset, BdecodeTokenType::End, next_item, 0)
    }

    pub fn new_str(offset: u32, head_size: u8) -> Self {
        let next_item = 1;
        Self::new_all(offset, BdecodeTokenType::Str, next_item, head_size)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_token_type() {
        assert_eq!(BdecodeTokenType::from_bits(1), BdecodeTokenType::Dict);
        assert_eq!(1, BdecodeTokenType::from_bits(1) as u8);
    }
}

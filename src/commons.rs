/// 阈值常量
pub mod limits {
    /// buffer 的最大长度，也就是 Token 中 offset 的最大值。
    pub const MAX_OFFSET: usize = (1 << 29) - 1;
    
    /// 下一个 Token 相对位置的最大值。
    pub const MAX_NEXT_ITEM: usize = (1 << 29) - 1;

    ///指 Dict 中组成 key 字符串的最大长度。
    pub const MAX_HEADER: usize = 7;
}

/// 节点类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NodeType {
    /// 不存在的节点类型
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

impl NodeType {
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_node_type() {
        assert_eq!(NodeType::from_bits(1), NodeType::Dict);
        assert_eq!(1, NodeType::from_bits(1) as u8);
    }
}
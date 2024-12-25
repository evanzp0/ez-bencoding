use bitfields::bitfield;

use crate::NodeType;

/// Bdecode 分词
/// 用来结构化描述 buffer 中 bencoding 编码的字符串
#[bitfield(u64)]
pub struct BdecodeToken {
    /// 当前节点在 bdecoded buffer 中对应的偏移位置
    #[bits(29)]
    offset: u32,

    /// 当前节点类型
    #[bits(3)]
    node_type: NodeType,

    /// 下一个节点在 tokens vector 中相对于当前节点的偏移索引值
    #[bits(29)]
    next_item: u32,

    /// 字符串在 bdecoded buffer 中需要跳过的头部字节数
    /// 
    /// 例如：
    /// "10:abcdefghij" 中的 header 值是 '10', 所以 header_size 为 2
    #[bits(3)]
    header_size: u8,
}

#[cfg(test)]
mod tests {
    use bitfields::bitfield;

    use crate::NodeType;

    #[bitfield(u8)]
    struct DisplayControl {
        #[bits(2)]
        bg_mode: u8,
        #[bits(6)]
        node_type: NodeType,
    }

    #[test]
    fn test_displaycontrol() {
        let mut dc = DisplayControl::new();
        dc.set_bg_mode(2);
        dc.set_node_type(NodeType::Dict);

        assert_eq!(2, dc.bg_mode());
        assert_eq!(NodeType::Dict, dc.node_type());
    }
}

use std::sync::Arc;

use crate::BdecodeToken;

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
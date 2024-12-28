use super::token::{BdecodeToken, BdecodeTokenType};

pub trait IBdecodeNode {
    fn token_index(&self) -> usize;
    fn tokens(&self) -> std::sync::Arc<Vec<BdecodeToken>>;
    
    /// 获取当前节点的 token 的类型
    fn token_type(&self) -> BdecodeTokenType {
        self.tokens()[self.token_index() as usize].node_type()
    }
}
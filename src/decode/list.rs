use std::borrow::Cow;

use crate::decode::token::BdecodeTokenType;

use super::{BdecodeNode, BdecodeResult, IBdecodeNode};

crate::collective_bdecode_node!(List);

impl List {

    /// 获取 list 中指定索引的节点
    pub fn item(&self, index: usize) -> BdecodeNode {
        assert!(self.token_type() == BdecodeTokenType::List);

        if index >= self.len() {
            // println!("index: {}, len: {}", index, self.len());
            panic!("index out of range");
        }

        let token_idx = self.item_indexes[index];
        BdecodeNode::new(token_idx, self.tokens.clone(), self.buffer.clone())
    }

    pub fn as_int(&self, index: usize) -> BdecodeResult<i64> {
        self.item(index).as_int()
    }

    pub fn as_str(&self, index: usize) -> Cow<[u8]> {
        let node = self.item(index);
        let val = node.as_str();

        let val_ptr = val.as_ref() as *const [u8];
        let val_ref = unsafe { &*val_ptr };

        Cow::Borrowed(val_ref)
    }

    pub fn to_json(&self) -> String {
        let mut sb = String::new();
        let len = self.len();

        for i in 0..len {
            let val = self.item(i);
            sb.push_str(&val.to_json());

            if i < len - 1 { 
                sb.push_str(", "); 
            }
        }
        
        format!("[{}]", sb)
    }
}

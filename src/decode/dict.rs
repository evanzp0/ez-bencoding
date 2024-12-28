use std::{borrow::Cow, collections::HashMap};

use super::{BdecodeNode, token::BdecodeTokenType, IBdecodeNode};

crate::collective_bdecode_node!(Dict);

impl Dict {
    /// 获取 dict 中指定索引的节点对(key, value)
    pub fn item(&self, index: usize) -> (BdecodeNode, BdecodeNode) {
        assert!(self.token_type() == BdecodeTokenType::Dict);

        if index >= self.len() {
            panic!("index out of range");
        }

        // get key node
        let key_token_idx = self.item_indexes[index];
        if key_token_idx as usize >= self.tokens.len() {
            panic!("index out of range in tokens");
        }
        let key_node = BdecodeNode::new(key_token_idx, self.tokens(), self.buffer.clone());
        let key_token = &self.tokens[key_token_idx as usize];
        
        // get value node
        let val_token_idx = key_token_idx + key_token.next_item();
        let val_node = BdecodeNode::new(val_token_idx, self.tokens(), self.buffer.clone());

        (key_node, val_node)
    }

    /// 在 dict 中查找 key 对应的 value
    pub fn find(&self, key: &[u8]) -> Option<BdecodeNode> {
        assert!(self.token_type() == BdecodeTokenType::Dict);

        for token_index in self.item_indexes.as_ref() {
            let token = &self.tokens[*token_index as usize];
            assert!(token.node_type() == BdecodeTokenType::Str);
            let next_offset = self.tokens[(token_index + 1) as usize].offset() as usize;
            let start = (token.offset() + token.header_size() as u32 + 1) as usize;

            if &self.buffer[start..next_offset] == key {
                let val_token_idx = *token_index + token.next_item();

                return Some(BdecodeNode::new(val_token_idx, self.tokens(), self.buffer.clone()));
            }
        }

        None
    }

    pub fn find_as_str(&self, key: &[u8]) -> Option<Cow<[u8]>> {
        let node = self.find(key);

        if let Some(node) = node {
            let val = node.as_str();
            let val_ptr = val.as_ref() as *const [u8];
            let val_ref = unsafe { &*val_ptr };

            let rst = Cow::Borrowed(val_ref);

            return Some(rst);
        }

        None
    }

    pub fn find_as_int(&self, key: &[u8]) -> Option<i64> {
        let node = self.find(key);

        if let Some(node) = node {
            return node.as_int().ok();
        }

        None
    }

    pub fn find_as_list(&self, key: &[u8]) -> Option<Vec<BdecodeNode>> {
        let node = self.find(key);

        if let Some(node) = node {
            return if let BdecodeNode::List(node) = node {
                let mut nodes = vec![];
                for i in 0..node.len() {
                    let node = node.item(i);
                    nodes.push(node);
                }

                Some(nodes)
            } else {
                None
            };
        }

        None
    }

    pub fn find_as_dict(&self, key: &[u8]) -> Option<HashMap<Cow<[u8]>, BdecodeNode>> {
        let Some(node) = self.find(key) else {
            return None;
        };

        let mut node_map = HashMap::new();
        let BdecodeNode::Dict(node) = node else { return None };

        for i in 0..node.len() {
            let (key, value) = node.item(i);

            let key_str = key.as_str();
            let key_ptr = key_str.as_ref() as *const [u8];
            let key_ref = unsafe { &*key_ptr };

            let key = Cow::Borrowed(key_ref);

            node_map.insert(key, value);
        }

        Some(node_map)
    }

    pub fn to_json(&self) -> String {
        let mut sb = String::new();
        let len = self.len();

        for i in 0..len {
            let (key, val) = self.item(i);
            sb.push_str(&format!("{}: {}", key.to_json(), val.to_json()));

            if i < len - 1 { 
                sb.push_str(", "); 
            }
        }
        
        format!("{} {} {}", "{", sb, "}")
    }
}

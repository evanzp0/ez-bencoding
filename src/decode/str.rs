use std::borrow::Cow;

use super::{token::BdecodeTokenType, IBdecodeNode};

crate::primitive_bdecode_node!(Str);

impl Str {
    /// 获取当前节点的字符串值
    pub fn value(&self) -> Cow<[u8]> {
        assert!(self.token_type() == BdecodeTokenType::Str);

        let token = &self.tokens[self.token_index as usize];
        let start = token.offset() as usize;
        let header_size = token.header_size() as usize + 1;
        let end = self.tokens[(self.token_index + 1) as usize].offset() as usize;

        let buf = &self.buffer[start + header_size..end];
        let rst = Cow::Borrowed(buf);

        rst
    }

    pub fn to_json(&self) -> String {
        let val = String::from_utf8_lossy_owned(self.value().into_owned());
        format!(r#""{}""#, val)
    }
}

use super::{token::BdecodeTokenType, utils::parse_uint, BdecodeResult, IBdecodeNode, Style};

crate::primitive_bdecode_node!(Int);

impl Int {
    /// 获取当前节点的整数值
    pub fn value(&self) -> BdecodeResult<i64> {
        assert!(self.token_type() == BdecodeTokenType::Int);

        let token_idx = self.token_index as usize;
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

    pub fn to_json_with_style(&self, _style: Style) -> String {
        self.value().expect("parse to int failed").to_string()
    }

}
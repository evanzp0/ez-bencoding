use std::borrow::Cow;

use crate::decode::{commons::IDENT_LEN, token::BdecodeTokenType};

use super::{utils::gen_blanks, BdecodeNode, BdecodeResult, IBdecodeNode, Style};

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

    pub fn to_json_with_style(&self, style: Style) -> String {
        let mut sb = String::new();
        let len = self.len();

        for i in 0..len {
            let val = self.item(i);
            if let Style::Pretty(span) = style {
                let span = span + IDENT_LEN;
                let blanks = gen_blanks(span);
                let val = val.to_json_with_style(Style::Pretty(span));
                sb.push_str(&format!("{blanks}{val}"));
            } else {
                sb.push_str(&val.to_json_with_style(Style::Compact));
            }

            if i < len - 1 { 
                sb.push_str(","); 
                if Style::Compact == style {
                    sb.push_str(" "); 
                } else {
                    sb.push_str("\n");
                }
            }
        }

        if let Style::Pretty(span) = style {
            let blanks = gen_blanks(span);
            format!("[\n{}\n{blanks}]", sb)
        } else {
            format!("[{}]", sb)
        }

        // let mut rst = BytesMut::new();
        // if let Style::Pretty(span) = style {
        //     rst.extend_from_slice(b"[\n");
        //     rst.extend(sb);
        //     rst.extend(b"\n");
        //     let blanks = gen_blanks(span);
        //     rst.extend(blanks);
        //     rst.extend(b"]");
        // }  else {
        //     rst.extend_from_slice(b"[");
        //     rst.extend(sb);
        //     rst.extend(b"]");
        // }

        // rst.into()
    }
}

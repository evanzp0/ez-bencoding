
use super::Style;

crate::primitive_bdecode_node!(End);

impl End {

    pub fn to_json_with_style(&self, _style: Style) -> String {
        "".to_string()
    }
}
/// 阈值常量
pub mod limits {
    /// buffer 的最大长度，也就是 Token 中 offset 的最大值。
    pub const BUFFER_MAX_OFFSET: usize = (1 << 29) - 1;
    
    /// 下一个 Token 相对位置的最大值。
    pub const MAX_NEXT_ITEM: usize = (1 << 29) - 1;

    ///指 Dict 中组成 key 字符串的最大长度。
    pub const MAX_HEADER_SIZE: usize = 7;

    /// 解析时 dict 和 list 的最大嵌套深度限制。
    pub const DEFAULT_DEPTH_LIMIT: usize = 100;

    /// 解析时 token 最大数量。
	pub const DEFAULT_TOKEN_LIMIT: i32 = 1000000;
}

pub const IDENT_LEN: usize = 4;
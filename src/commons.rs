/// 阈值常量
pub mod limits {
    /// buffer 的最大长度，也就是 Token 中 offset 的最大值。
    pub const MAX_OFFSET: usize = (1 << 29) - 1;
    
    /// 下一个 Token 相对位置的最大值。
    pub const MAX_NEXT_ITEM: usize = (1 << 29) - 1;

    ///指 Dict 中组成 key 字符串的最大长度。
    pub const MAX_HEADER: usize = 7;
}

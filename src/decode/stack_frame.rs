use bitfields::bitfield;

/// 解析 dict 或 list 过程中的会生成一个 stack, 
/// 其中每一个 StackFrame 元素都对应 dict 或 list 的入口 token .
#[bitfield(u32)]
#[derive(Clone, Copy)]
pub(crate) struct StackFrame {
    #[bits(31)]
    token: u32,

    #[bits(1)]
    state: u8,
}
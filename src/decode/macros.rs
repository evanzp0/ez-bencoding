
#[macro_export]
macro_rules! primitive_bdecode_node {
    ($node_name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $node_name {
            /// 当前节点在 tokens 中的索引
            /// 0 - root 节点值; -1 - 未初始化
            pub token_index: u32,

            /// 解析后的 token 集合
            pub tokens: std::sync::Arc<Vec<super::token::BdecodeToken>>,

            /// 存放解析前字符串的 buffer
            pub buffer: std::sync::Arc<Vec<u8>>,
        }

        impl $node_name {
            pub fn new(
                buffer: std::sync::Arc<Vec<u8>>,
                tokens: std::sync::Arc<Vec<super::token::BdecodeToken>>,
                token_index: u32,
            ) -> Self {
                Self {
                    buffer,
                    tokens,
                    token_index,
                }
            }
        }

        impl super::IBdecodeNode for $node_name {
            fn token_index(&self) -> usize {
                self.token_index as usize
            }

            fn tokens(&self) -> std::sync::Arc<Vec<super::token::BdecodeToken>> {
                self.tokens.clone()
            }
        }
    };
}

#[macro_export]
macro_rules! collective_bdecode_node {
    ($node_name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $node_name {
            /// 当前节点在 tokens 中的索引
            /// 0 - root 节点值; -1 - 未初始化
            pub token_index: u32,

            /// 解析后的 token 集合
            pub tokens: std::sync::Arc<Vec<super::token::BdecodeToken>>,

            /// 存放解析前字符串的 buffer
            pub buffer: std::sync::Arc<Vec<u8>>,

            /// 存放 list 和 map 中 item 的对应的 token 索引集合
            pub item_indexes: std::sync::Arc<Vec<u32>>,
            // pub item_begin_len: Option<(u32, u32)>,

            /// list 和 map 中 item 的数量
            len: usize,
        }

        impl $node_name {
            pub fn new(
                buffer: std::sync::Arc<Vec<u8>>,
                tokens: std::sync::Arc<Vec<super::token::BdecodeToken>>,
                token_index: u32,
                item_indexes: std::sync::Arc<Vec<u32>>,
                len: usize,
            ) -> Self {
                Self {
                    buffer,
                    tokens,
                    token_index,
                    item_indexes,
                    len,
                }
            }

            /// 获取当前 list or dict 节点的长度
            pub fn len(&self) -> usize {
                use crate::IBdecodeNode;
                use super::token::BdecodeTokenType::*;

                assert!(matches!(self.token_type(), Dict | List));

                self.len
            }
        }

        impl super::IBdecodeNode for $node_name {
            fn token_index(&self) -> usize {
                self.token_index as usize
            }

            fn tokens(&self) -> std::sync::Arc<Vec<super::token::BdecodeToken>> {
                self.tokens.clone()
            }
        }
    }
}

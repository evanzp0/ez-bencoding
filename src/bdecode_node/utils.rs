

// mod display {
//     use super::*;

//     fn str_to_string(token: &BdecodeToken, buffer: &[u8]) -> String {
//         let start = token.offset() as usize;
//         let len = token.header_size() as usize;
//         format!("\"{}\"", String::from_utf8_lossy(&buffer[start..(start + len)]))
//     }

//     fn int_to_string(token: &BdecodeToken, buffer: &[u8]) -> String {
//         let start = token.offset() as usize;
//         let len = token.header_size() as usize;
//         format!("{}", String::from_utf8_lossy(&buffer[start..(start + len)]))
//     }

//     fn dict_to_string(token: &BdecodeToken, buffer: &[u8]) -> String {
//         format!("{{{}}}", str_to_string(token, buffer))
//     }

//     fn list_to_string(token: &BdecodeToken, buffer: &[u8]) -> String {
//         format!("[{}]", str_to_string(token, buffer))
//     }
// }

// BdecodeNode {
//     tokens: [
//         BdecodeToken {
//             header_size: 0,
//             next_item: 8,
//             node_type: 1,
//             offset: 0,
//         },
//         BdecodeToken {
//             header_size: 1,
//             next_item: 1,
//             node_type: 3,
//             offset: 1,
//         },
//         BdecodeToken {
//             header_size: 1,
//             next_item: 1,
//             node_type: 3,
//             offset: 4,
//         },
//         BdecodeToken {
//             header_size: 1,
//             next_item: 1,
//             node_type: 3,
//             offset: 7,
//         },
//         BdecodeToken {
//             header_size: 1,
//             next_item: 1,
//             node_type: 3,
//             offset: 11,
//         },
//         BdecodeToken {
//             header_size: 1,
//             next_item: 1,
//             node_type: 3,
//             offset: 16,
//         },
//         BdecodeToken {
//             header_size: 0,
//             next_item: 1,
//             node_type: 4,
//             offset: 22,
//         },
//         BdecodeToken {
//             header_size: 0,
//             next_item: 1,
//             node_type: 5,
//             offset: 25,
//         },
//         BdecodeToken {
//             header_size: 0,
//             next_item: 1,
//             node_type: 5,
//             offset: 26,
//         },
//     ],
//     buffer: [
//         100,
//         49,
//         58,
//         97,
//         49,
//         58,
//         98,
//         50,
//         58,
//         99,
//         100,
//         51,
//         58,
//         102,
//         111,
//         111,
//         52,
//         58,
//         98,
//         97,
//         114,
//         111,
//         105,
//         57,
//         101,
//         101,
//     ],
//     token_idx: 0,
//     item_begin_len: Some(
//         (
//             1,
//             6,
//         ),
//     ),
// }
use std::sync::Arc;

use crate::{BdecodeError, BdecodeResult};

use super::token::{BdecodeToken, BdecodeTokenType};

/// 为一个 Bdecode 节点生成它的子节点的索引列表，以及长度。
pub(crate) fn gen_item_indexes(
    tokens: &[BdecodeToken],
    start_token_idx: usize,
) -> (Arc<Vec<u32>>, usize) {
    use BdecodeTokenType::*;

    assert!(start_token_idx < tokens.len());

    if tokens.len() < 2 {
        return (Default::default(), 0);
    }

    let mut node_indexes = vec![];
    let mut count = 0;

    let mut begin = 1 + start_token_idx;
    match tokens[start_token_idx].node_type() {
        Dict => {
            while tokens[begin].node_type() != End {
                if count % 2 == 0 {
                    node_indexes.push(begin as u32);
                }
                count += 1;

                begin += tokens[begin].next_item() as usize;
            }
            count /= 2;
        }
        List => {
            while tokens[begin].node_type() != End {
                node_indexes.push(begin as u32);
                begin += tokens[begin].next_item() as usize;
                count += 1;
            }
        }
        _ => (),
    }

    (Arc::new(node_indexes), count)
}

/// 检查字符串是否为整数
pub(crate) fn check_integer(buffer: &[u8], start: usize) -> BdecodeResult<usize> {
    let mut start = start as usize;
    let orgin_start = start;
    let end = buffer.len();

    if buffer.is_empty() {
        return Err(BdecodeError::UnexpectedEof(0));
    }

    if start >= end {
        return Err(BdecodeError::UnexpectedEof(start));
    }

    if buffer[start] == b'-' {
        start += 1;
        if start == end {
            return Err(BdecodeError::UnexpectedEof(start));
        }
    }

    let mut digits = 0;
    while buffer[start] != b'e' {
        let t = buffer[start];

        if !t.is_ascii_digit() {
            println!("!!!!!");
            return Err(BdecodeError::ExpectedDigit(start));
        }
        start += 1;
        digits += 1;

        if start >= end {
            return Err(BdecodeError::UnexpectedEof(start));
        }
    }

    if digits > 20 {
        let msg = String::from_utf8_lossy_owned(buffer[orgin_start..start].to_vec());
        return Err(BdecodeError::Overflow(msg));
    }

    Ok(start)
}

/// 解析 buffer 中的整数, 当遇到 delimiter 字符时停止解析
pub(crate) fn parse_uint(
    buffer: &[u8],
    mut start: usize,
    delimiter: u8,
    val: &mut i64,
) -> BdecodeResult<usize> {
    let end = buffer.len();

    while start < end && buffer[start] != delimiter {
        let t = buffer[start];

        if !t.is_ascii_digit() {
            println!("is_ascii_digit !!!!!");
            return Err(BdecodeError::ExpectedDigit(start));
        }

        // 检查 val * 10 是否会溢出
        if *val > i64::MAX / 10 {
            return Err(BdecodeError::Overflow(format!("{val}0")));
        }
        *val *= 10;

        let digit = (t - b'0') as i64;
        // 检查 val + digit 是否会溢出
        if *val > i64::MAX - digit {
            return Err(BdecodeError::Overflow(format!("{}", *val + digit)));
        }

        *val += digit;
        start += 1;
    }

    Ok(start)
}

pub(crate) fn gen_blanks(span: usize) -> String {
    if span == 0 {
        "".into()
    } else {
        " ".repeat(span).into()
    }
}

pub fn escape_char(byte: u8) -> String {
    match byte {
        b' ' => " ".into(),
        b'"' => format!("\\x{:02x}", byte),
        _ if byte.is_ascii_graphic() =>  format!("{}", byte as char),
        _ => format!("\\x{:02x}", byte),
    }
    .to_string()
}

pub fn escape_string(bytes: &[u8]) -> String {
    let mut result = String::new();
    for c in bytes.iter() {
        result.push_str(&escape_char(*c));
    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let buffer = b"i1234e";
        let mut val: i64 = 0;
        assert_eq!(5, parse_uint(buffer, 1, b'e', &mut val).unwrap());
        assert_eq!(1234, val);

        let buffer = b"1234:i2e";
        let mut val: i64 = 1;
        assert_eq!(4, parse_uint(buffer, 1, b':', &mut val).unwrap());
        assert_eq!(1234, val);

        let buffer = b"d1234:i2e";
        let mut val: i64 = 0;
        assert!(matches!(
            parse_uint(buffer, 0, b':', &mut val),
            Err(BdecodeError::ExpectedDigit(_))
        ));
    }

    #[test]
    fn test_check_integer() {
        let buffer = b"i-11e";
        assert_eq!(4, check_integer(buffer, 1).unwrap());

        let buffer = b"i1234e";
        let err = check_integer(buffer, 0).unwrap_err();
        assert!(matches!(err, BdecodeError::ExpectedDigit(_)));

        let buffer = b"i1234e";
        assert_eq!(5, check_integer(buffer, 1).unwrap());

        let buffer = b"i012345678901234567890123456789e";
        let err = check_integer(buffer, 1).unwrap_err();
        assert!(matches!(err, BdecodeError::Overflow(_)));

        let buffer = b"";
        let err = check_integer(buffer, 1).unwrap_err();
        assert!(matches!(err, BdecodeError::UnexpectedEof(_)));

        let buffer = b"i1234e";
        let err = check_integer(buffer, 6).unwrap_err();
        assert!(matches!(err, BdecodeError::UnexpectedEof(_)));
    }

    #[test]
    fn test_gen_item_indexes() {
        // 2:v1
        let v_1 = BdecodeToken::new_str(0, 1);
        let e_x = BdecodeToken::new_end(1);
        let tokens = vec![v_1, e_x];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![]), 0));

        // {"k1": "v1", "k2": [1, 2], "k3": 3}
        // str  | pos   | seq
        // --------------------
        // d_1  | 0     | 0
        // 2:k1 | 1     | 1
        // 2:v1 | 5     | 2
        // 2:k2 | 9     | 3
        // l_2  | 13    | 4
        // i1e  | 14    | 5
        // i2e  | 17    | 6
        // e_2  | 20    | 7
        // 2:k3 | 21    | 8
        // i3e  | 25    | 9
        // e_1  | 28    | 10
        // e_x  | 29    | 11
        let d_1 = BdecodeToken::new_dict(0, 11);
        let k_1 = BdecodeToken::new_str(1, 1);
        let v_1 = BdecodeToken::new_str(5, 1);
        let k_2 = BdecodeToken::new_str(9, 1);
        let l_2 = BdecodeToken::new_list(13, 4);
        let i_1 = BdecodeToken::new_int(14);
        let i_2 = BdecodeToken::new_int(17);
        let e_2 = BdecodeToken::new_end(20);
        let k_3 = BdecodeToken::new_str(21, 1);
        let i_3 = BdecodeToken::new_int(25);
        let e_1 = BdecodeToken::new_end(28);
        let e_x = BdecodeToken::new_end(29);
        let tokens = vec![ d_1, k_1, v_1, k_2, l_2, i_1, i_2, e_2, k_3, i_3, e_1, e_x ];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![1, 3, 8]), 3));

        // [1, [2], {"k4": 4}]
        // str  | pos   | seq
        // --------------------
        // l_1  | 0     | 0
        // i1e  | 1     | 1
        // l_2  | 4     | 2
        // i2e  | 5     | 3
        // e_2  | 8     | 4
        // d_3  | 9     | 5
        // 2:k4 | 10    | 6
        // i4e  | 14    | 7
        // e_3  | 17    | 8
        // e_x  | 18    | 9
        let l_1 = BdecodeToken::new_list(0, 9);
        let i_1 = BdecodeToken::new_int(1);
        let l_2 = BdecodeToken::new_list(4, 3);
        let i_2 = BdecodeToken::new_int(5);
        let e_2 = BdecodeToken::new_end(8);
        let d_3 = BdecodeToken::new_dict(9, 4);
        let k_4 = BdecodeToken::new_str(10, 1);
        let i_4 = BdecodeToken::new_int(14);
        let e_3 = BdecodeToken::new_end(17);
        let e_x = BdecodeToken::new_end(18);
        let tokens = vec![l_1, i_1, l_2, i_2, e_2, d_3, k_4, i_4, e_3, e_x];
        let rst = gen_item_indexes(&tokens, 0);
        assert_eq!(rst, (Arc::new(vec![1, 2, 5]), 3));
    }
}

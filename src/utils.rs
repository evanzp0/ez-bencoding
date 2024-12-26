use crate::{BdecodeError, BdecodeResult};

/// 检查字符串是否为整数
pub fn check_integer(buffer: &[u8], start: usize) -> BdecodeResult<usize> {
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
pub fn parse_uint(buffer: &[u8], mut start: usize, delimiter: u8, val: &mut i64) -> BdecodeResult<usize> {
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
        assert!(matches!(parse_uint(buffer, 0, b':', &mut val), Err(BdecodeError::ExpectedDigit(_))));
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
}
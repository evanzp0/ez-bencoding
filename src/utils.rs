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

    while buffer[start] != b'e' {
        let t = buffer[start];

        if !t.is_ascii_digit() {
            return Err(BdecodeError::ExpectedDigit(start));
        }
        start += 1;

        if start >= end {
            return Err(BdecodeError::UnexpectedEof(start));
        }
    }

    if start > 20 {
        let msg = String::from_utf8_lossy_owned(buffer[orgin_start..start].to_vec());
        return Err(BdecodeError::Overflow(msg));
    }

    Ok(start)
}

#[cfg(test)]
mod tests {
    use super::*;

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
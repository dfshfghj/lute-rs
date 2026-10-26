// Lute - 一款结构化的 Markdown 引擎，支持 Go 和 JavaScript
// Copyright (c) 2019-present, b3log.org
//
// Lute is licensed under Mulan PSL v2.

//! Base64 编码，对应 Go 版 encoding/base64。

const STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn encode_with(tokens: &[u8], alphabet: &[u8; 64], pad: bool) -> String {
    let mut ret = String::with_capacity(tokens.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= tokens.len() {
        let n = ((tokens[i] as u32) << 16) | ((tokens[i + 1] as u32) << 8) | tokens[i + 2] as u32;
        ret.push(alphabet[(n >> 18) as usize & 0x3F] as char);
        ret.push(alphabet[(n >> 12) as usize & 0x3F] as char);
        ret.push(alphabet[(n >> 6) as usize & 0x3F] as char);
        ret.push(alphabet[n as usize & 0x3F] as char);
        i += 3;
    }
    let rem = tokens.len() - i;
    if 1 == rem {
        let n = (tokens[i] as u32) << 16;
        ret.push(alphabet[(n >> 18) as usize & 0x3F] as char);
        ret.push(alphabet[(n >> 12) as usize & 0x3F] as char);
        if pad {
            ret.push('=');
            ret.push('=');
        }
    } else if 2 == rem {
        let n = ((tokens[i] as u32) << 16) | ((tokens[i + 1] as u32) << 8);
        ret.push(alphabet[(n >> 18) as usize & 0x3F] as char);
        ret.push(alphabet[(n >> 12) as usize & 0x3F] as char);
        ret.push(alphabet[(n >> 6) as usize & 0x3F] as char);
        if pad {
            ret.push('=');
        }
    }
    ret
}

/// 标准 Base64（带填充），对应 base64.StdEncoding。
pub fn encode_std(tokens: &[u8]) -> String {
    encode_with(tokens, STD, true)
}

/// URL 安全 Base64（不带填充），对应 base64.RawURLEncoding。
pub fn encode_raw_url(tokens: &[u8]) -> String {
    encode_with(tokens, URL, false)
}

fn decode_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

/// URL 安全 Base64 解码（不带填充），对应 base64.RawURLEncoding.DecodeString。
///
/// 与 Go 一致：忽略换行，拒绝填充符、非法字符、长度为 1 的余数以及非零尾部比特。
pub fn decode_raw_url(encoded: &str) -> Option<Vec<u8>> {
    let mut ret = Vec::with_capacity(encoded.len() / 4 * 3 + 2);
    let mut quantum = [0u8; 4];
    let mut count = 0;
    for b in encoded.bytes() {
        if b'\r' == b || b'\n' == b {
            continue;
        }
        quantum[count] = decode_value(b)?;
        count += 1;
        if 4 == count {
            ret.push((quantum[0] << 2) | (quantum[1] >> 4));
            ret.push((quantum[1] << 4) | (quantum[2] >> 2));
            ret.push((quantum[2] << 6) | quantum[3]);
            count = 0;
        }
    }
    match count {
        0 => {}
        // Go 的 Raw 解码器把长度为 1 的余数视为非法输入。
        1 => return None,
        2 => {
            if 0 != quantum[1] & 0x0F {
                return None;
            }
            ret.push((quantum[0] << 2) | (quantum[1] >> 4));
        }
        _ => {
            if 0 != quantum[2] & 0x03 {
                return None;
            }
            ret.push((quantum[0] << 2) | (quantum[1] >> 4));
            ret.push((quantum[1] << 4) | (quantum[2] >> 2));
        }
    }
    Some(ret)
}

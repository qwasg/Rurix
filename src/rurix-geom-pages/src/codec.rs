//! RXPZ-LZ1 — 手写确定性字节向 LZ77（LZ4-block 风格；RXS-0340）。
//! 禁 zstd/flate2。

/// codec_id 注册表字面（RXS-0339）。
pub const CODEC_ID_RXPZ_LZ1: u32 = 1;
pub const CODEC_VERSION: u32 = 1;

const MINMATCH: usize = 4;
const LASTLITERALS: usize = 5;
const MFLIMIT: usize = 12;
const WINDOW: usize = 64 * 1024;
const HASH_LOG: usize = 12;
const HASH_SIZE: usize = 1 << HASH_LOG;
const MAX_CHAIN: usize = 64;

/// 压缩/解压错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    Truncated(&'static str),
    Corrupt(&'static str),
    SizeMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::Truncated(s) => write!(f, "RXPZ-LZ1 截断:{s}"),
            CodecError::Corrupt(s) => write!(f, "RXPZ-LZ1 损坏:{s}"),
            CodecError::SizeMismatch { expected, got } => {
                write!(f, "RXPZ-LZ1 尺寸不符:期望{expected} 得{got}")
            }
        }
    }
}

impl std::error::Error for CodecError {}

fn hash4(src: &[u8], i: usize) -> usize {
    let v = u32::from_le_bytes([src[i], src[i + 1], src[i + 2], src[i + 3]]);
    (v.wrapping_mul(2654435761) >> (32 - HASH_LOG)) as usize
}

/// 确定性压缩：贪心最长匹配；hash 链按插入序回退，链深 ≤64。
pub fn compress(src: &[u8]) -> Vec<u8> {
    if src.is_empty() {
        return vec![0];
    }
    let mut out = Vec::with_capacity(src.len() + src.len() / 8 + 16);
    let mut head = [usize::MAX; HASH_SIZE];
    let mut chain = vec![usize::MAX; src.len()];
    let mut anchor = 0usize;
    let mut ip = 0usize;
    let match_limit = src.len().saturating_sub(LASTLITERALS);
    let mflimit_end = src.len().saturating_sub(MFLIMIT);

    while ip < mflimit_end {
        let h = hash4(src, ip);
        let mut best_off = 0usize;
        let mut best_len = 0usize;
        let mut cand = head[h];
        let window_lo = ip.saturating_sub(WINDOW);
        let mut depth = 0usize;
        while cand != usize::MAX && cand >= window_lo && depth < MAX_CHAIN {
            depth += 1;
            if src[cand..cand + MINMATCH] == src[ip..ip + MINMATCH] {
                let mut len = MINMATCH;
                let max = match_limit - ip;
                while len < max && src[cand + len] == src[ip + len] {
                    len += 1;
                }
                // 更长优先；同长取较小 offset（更近）——确定性。
                let off = ip - cand;
                if len > best_len || (len == best_len && off < best_off) {
                    best_len = len;
                    best_off = off;
                }
            }
            cand = chain[cand];
        }
        // 插入当前位到链头。
        chain[ip] = head[h];
        head[h] = ip;

        if best_len >= MINMATCH && best_off > 0 && best_off <= 65535 {
            emit_sequence(&mut out, &src[anchor..ip], best_off as u16, best_len);
            let match_end = ip + best_len;
            ip += 1;
            while ip < match_end && ip < mflimit_end {
                let hh = hash4(src, ip);
                chain[ip] = head[hh];
                head[hh] = ip;
                ip += 1;
            }
            ip = match_end;
            anchor = ip;
        } else {
            ip += 1;
        }
    }

    emit_last_literals(&mut out, &src[anchor..]);
    out
}

fn emit_sequence(out: &mut Vec<u8>, literals: &[u8], offset: u16, match_len: usize) {
    let lit_len = literals.len();
    let match_code = match_len - MINMATCH;
    let token_lit = lit_len.min(15);
    let token_mat = match_code.min(15);
    out.push(((token_lit as u8) << 4) | (token_mat as u8));
    write_extra_len(out, lit_len, 15);
    out.extend_from_slice(literals);
    out.extend_from_slice(&offset.to_le_bytes());
    write_extra_len(out, match_code, 15);
}

fn emit_last_literals(out: &mut Vec<u8>, literals: &[u8]) {
    let lit_len = literals.len();
    let token_lit = lit_len.min(15);
    out.push((token_lit as u8) << 4);
    write_extra_len(out, lit_len, 15);
    out.extend_from_slice(literals);
}

fn write_extra_len(out: &mut Vec<u8>, len: usize, lim: usize) {
    if len < lim {
        return;
    }
    let mut remain = len - lim;
    while remain >= 255 {
        out.push(255);
        remain -= 255;
    }
    out.push(remain as u8);
}

/// 解压到恰好 `expected_size` 字节。
pub fn decompress(src: &[u8], expected_size: usize) -> Result<Vec<u8>, CodecError> {
    let mut out = Vec::with_capacity(expected_size);
    let mut ip = 0usize;
    if src.is_empty() {
        return if expected_size == 0 {
            Ok(out)
        } else {
            Err(CodecError::Truncated("empty"))
        };
    }
    while ip < src.len() {
        let token = src[ip];
        ip += 1;
        let mut lit_len = (token >> 4) as usize;
        if lit_len == 15 {
            lit_len += read_extra(src, &mut ip)?;
        }
        if ip + lit_len > src.len() {
            return Err(CodecError::Truncated("literals"));
        }
        if out.len() + lit_len > expected_size {
            return Err(CodecError::SizeMismatch {
                expected: expected_size,
                got: out.len() + lit_len,
            });
        }
        out.extend_from_slice(&src[ip..ip + lit_len]);
        ip += lit_len;

        if ip >= src.len() {
            break;
        }
        if ip + 2 > src.len() {
            return Err(CodecError::Truncated("offset"));
        }
        let offset = u16::from_le_bytes([src[ip], src[ip + 1]]) as usize;
        ip += 2;
        if offset == 0 || offset > out.len() {
            return Err(CodecError::Corrupt("offset"));
        }
        let mut match_len = (token & 0x0f) as usize + MINMATCH;
        if (token & 0x0f) == 15 {
            match_len += read_extra(src, &mut ip)?;
        }
        if out.len() + match_len > expected_size {
            return Err(CodecError::SizeMismatch {
                expected: expected_size,
                got: out.len() + match_len,
            });
        }
        for _ in 0..match_len {
            let b = out[out.len() - offset];
            out.push(b);
        }
    }
    if out.len() != expected_size {
        return Err(CodecError::SizeMismatch {
            expected: expected_size,
            got: out.len(),
        });
    }
    Ok(out)
}

fn read_extra(src: &[u8], ip: &mut usize) -> Result<usize, CodecError> {
    let mut total = 0usize;
    loop {
        let b = *src.get(*ip).ok_or(CodecError::Truncated("extra_len"))?;
        *ip += 1;
        total += b as usize;
        if b != 255 {
            break;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0340
    #[test]
    fn roundtrip_and_deterministic() {
        let samples: &[&[u8]] = &[
            b"",
            b"a",
            b"aaaaaaaabbbbbbbbcccccccc",
            b"The quick brown fox jumps over the lazy dog. The quick brown fox!",
            &[7u8; 200],
        ];
        for s in samples {
            let a = compress(s);
            let b = compress(s);
            assert_eq!(a, b, "deterministic");
            let back = decompress(&a, s.len()).expect("dec");
            assert_eq!(&back[..], *s);
        }
    }
}

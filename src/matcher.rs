fn at(f: &[u8], i: usize) -> u8 {
    f.get(i).copied().unwrap_or(0)
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

fn match_skip(f: &[u8], mut i: usize, alternation: bool) -> usize {
    loop {
        let c = at(f, i);
        i += 1;
        match c {
            b'(' => i = match_skip(f, i, false) + 1,
            b'\\' => i += 1,
            b'[' => {
                let mut c = at(f, i);
                while c != b']' {
                    c = at(f, i);
                    i += 1;
                    if c == b'\\' {
                        c = at(f, i);
                        i += 1;
                    }
                }
                i += 1;
            }
            b'|' if alternation => return i - 1,
            b')' | 0 => return i - 1,
            _ => {}
        }
    }
}

const MACRO_DIGIT: &[u8] = b"[0-9]";
const MACRO_FLOAT: &[u8] = b"[\\-+]?[0-9]+(\\.[0-9]+)?([eE][\\-+]?[0-9]+)?";

fn match_imp(s: &[u8], p_str: &mut usize, f: &[u8], p_fmt: &mut usize) -> bool {
    let str_original_begin = *p_str;
    let mut str = str_original_begin;
    let mut fmt_begin = *p_fmt;
    let mut fmt = fmt_begin;
    let mut case_insensitive = false;
    let mut count = 0usize;
    loop {
        let c = at(f, fmt);
        fmt += 1;
        if c == 0 {
            *p_str = str;
            *p_fmt = fmt - 1;
            return true;
        }
        let str_begin = str;
        let mut r = if str < s.len() { s[str] } else { 0 };
        if case_insensitive && r.is_ascii_uppercase() {
            r = r - b'A' + b'a';
        }

        let mut ok = false;
        match c {
            b'\\' => {
                let c = at(f, fmt);
                fmt += 1;
                let mut mac: Option<&[u8]> = None;
                match c {
                    b'd' => mac = Some(MACRO_DIGIT),
                    b'F' => mac = Some(MACRO_FLOAT),
                    b's' => {
                        if is_space(r) {
                            ok = true;
                            str += 1;
                        }
                    }
                    b'S' => {
                        if !is_space(r) {
                            ok = true;
                            str += 1;
                        }
                    }
                    b'c' | b'C' => {
                        case_insensitive = c == b'c';
                        ok = true;
                    }
                    _ => {
                        if r == c {
                            ok = true;
                            str += 1;
                        }
                    }
                }
                if let Some(m) = mac {
                    let mut mp = 0;
                    ok = match_imp(s, &mut str, m, &mut mp);
                }
            }
            b'[' => {
                while at(f, fmt) != b']' {
                    if at(f, fmt) == b'\\' {
                        if r == at(f, fmt + 1) {
                            ok = true;
                        }
                        fmt += 2;
                    } else if at(f, fmt + 1) == b'-' {
                        if r >= at(f, fmt) && r <= at(f, fmt + 2) {
                            ok = true;
                        }
                        fmt += 3;
                    } else {
                        if r == at(f, fmt) {
                            ok = true;
                        }
                        fmt += 1;
                    }
                }
                fmt += 1;
                if ok {
                    str += 1;
                }
            }
            b'(' => {
                if match_imp(s, &mut str, f, &mut fmt) {
                    ok = true;
                }
            }
            b'|' => {
                fmt = match_skip(f, fmt, false);
                ok = true;
            }
            b')' => {
                *p_str = str;
                *p_fmt = fmt;
                return true;
            }
            b'.' => {
                if r != 0 {
                    ok = true;
                    str += 1;
                }
            }
            _ => {
                if c == r {
                    str += 1;
                    ok = true;
                }
            }
        }

        let mut did_fail = false;
        match at(f, fmt) {
            b'*' => {
                fmt += 1;
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                }
            }
            b'+' => {
                fmt += 1;
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                } else if count == 0 {
                    did_fail = true;
                }
            }
            b'?' => fmt += 1,
            _ => did_fail = !ok,
        }

        if did_fail {
            fmt = match_skip(f, fmt, true);
            if at(f, fmt) == b'|' {
                fmt += 1;
                str = str_original_begin;
            } else {
                *p_fmt = match_skip(f, fmt, false) + 1;
                return false;
            }
        } else if !ok {
            str = str_begin;
        }
        fmt_begin = fmt;
        count = 0;
    }
}

/// `ufbxi_match` in ufbx.c
pub(crate) fn match_pattern(s: &[u8], fmt: &str) -> bool {
    let (mut ps, mut pf) = (0, 0);
    match_imp(s, &mut ps, fmt.as_bytes(), &mut pf) && ps == s.len()
}

fn next_line<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
    if buf.is_empty() {
        return None;
    }
    let length = buf.iter().position(|&c| c == b'\n').map_or(buf.len(), |i| i + 1);
    let mut line = &buf[..length];
    *buf = &buf[length..];
    while let [first, rest @ ..] = line
        && is_space(*first)
    {
        line = rest;
    }
    while let [rest @ .., last] = line
        && is_space(*last)
    {
        line = rest;
    }
    Some(line)
}

pub(crate) fn is_format(data: &[u8], format: crate::types::FileFormat) -> bool {
    use crate::types::FileFormat;
    let mut buf = data;
    match format {
        FileFormat::Fbx => {
            if data.starts_with(crate::parse::BINARY_MAGIC) {
                return true;
            }
            while let Some(line) = next_line(&mut buf) {
                if match_pattern(line, ";\\s*FBX\\s*\\d+\\.\\d+\\.\\d+\\s*project\\s+file") || match_pattern(line, "FBXHeaderExtension:.*") {
                    return true;
                }
            }
        }
        FileFormat::Obj => {
            while let Some(line) = next_line(&mut buf) {
                if match_pattern(line, "(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*|f\\s+[\\-/0-9]+\\s+[\\-/0-9]+\\s*[\\-/0-9]+.*|(usemtl|mtllib)\\s+\\S.*") {
                    return true;
                }
            }
        }
        FileFormat::Mtl => {
            while let Some(line) = next_line(&mut buf) {
                if match_pattern(line, "newmtl\\s+\\S.*") {
                    return true;
                }
            }
        }
        FileFormat::Tds => return data.len() >= 6 && data[..2] == [0x4D, 0x4D],
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patterns_match_as_ufbx_matches_them() {
        assert!(match_pattern(b"; FBX 7.4.0 project file", ";\\s*FBX\\s*\\d+\\.\\d+\\.\\d+\\s*project\\s+file"));
        assert!(match_pattern(b"v 1.0 -2 3e5", "(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*"));
        assert!(match_pattern(b"vt 0.5 0.5", "(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*"));
        assert!(!match_pattern(b"vx 1 2 3", "(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*"));
        assert!(match_pattern(b"f 1/2/3 4//5 6", "f\\s+[\\-/0-9]+\\s+[\\-/0-9]+\\s*[\\-/0-9]+.*"));
        assert!(match_pattern(b".FBX", "\\c\\.fbx"));
        assert!(!match_pattern(b".fbxx", "\\c\\.fbx"));
        assert!(is_format(b"# x\nnewmtl a\n", crate::types::FileFormat::Mtl));
    }
}

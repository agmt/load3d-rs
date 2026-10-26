use crate::error::{Res, check, fail};

const CTYPE_WHITESPACE: u8 = 0x1;
const CTYPE_SINGLE_QUOTE: u8 = 0x2;
const CTYPE_DOUBLE_QUOTE: u8 = 0x4;
const CTYPE_NAME_END: u8 = 0x8;
const CTYPE_TAG_START: u8 = 0x10;
const CTYPE_END_OF_FILE: u8 = 0x20;

const MAX_XML_DEPTH: usize = 32;

/// `ufbxi_xml_ctype` in ufbx.c
fn ctype(c: u8) -> u8 {
    match c {
        0 => CTYPE_END_OF_FILE,
        b'\t' | b'\n' | b'\r' | b' ' => CTYPE_WHITESPACE | CTYPE_NAME_END,
        b'"' => CTYPE_DOUBLE_QUOTE | CTYPE_NAME_END,
        b'\'' => CTYPE_SINGLE_QUOTE | CTYPE_NAME_END,
        b'/' | b'=' | b'>' | b'?' => CTYPE_NAME_END,
        b'<' => CTYPE_TAG_START,
        _ => 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct XmlAttrib {
    pub name: Vec<u8>,
    pub value: Vec<u8>,
}

/// Text and CDATA are nameless tags among the children
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct XmlTag {
    pub name: Vec<u8>,
    pub text: Vec<u8>,
    pub attribs: Vec<XmlAttrib>,
    pub children: Vec<XmlTag>,
}

impl XmlTag {
    pub fn xml_find_child(&self, name: &str) -> Option<&XmlTag> {
        self.children.iter().find(|c| c.name == name.as_bytes())
    }

    pub fn xml_find_attrib(&self, name: &str) -> Option<&XmlAttrib> {
        self.attribs.iter().find(|a| a.name == name.as_bytes())
    }
}

/// The leading digits, wrapping as `uint32_t` does
pub(crate) fn parse_uint32_radix(s: &[u8], radix: u32) -> u32 {
    let mut value: u32 = 0;
    for &c in s {
        let digit = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' if radix == 16 => c - b'a' + 10,
            b'A'..=b'F' if radix == 16 => c - b'A' + 10,
            _ => break,
        };
        value = value.wrapping_mul(radix).wrapping_add(digit as u32);
    }
    value
}

/// ufbx streams the file and reads NUL past its end, here `cur` gives 0 there
struct XmlContext<'a> {
    data: &'a [u8],
    pos: usize,
    tok: Vec<u8>,
    tmp_stack: Vec<XmlTag>,
}

impl XmlContext<'_> {
    fn cur(&self) -> u8 {
        self.data.get(self.pos).copied().unwrap_or(0)
    }

    fn xml_advance(&mut self) {
        self.pos += 1;
    }

    fn xml_accept(&mut self, ch: u8) -> bool {
        if self.cur() == ch {
            self.xml_advance();
            true
        } else {
            false
        }
    }

    fn xml_skip_while(&mut self, ctypes: u8) {
        while ctype(self.cur()) & ctypes != 0 {
            self.xml_advance();
        }
    }

    fn xml_skip_until_string(&mut self, suffix: &[u8]) -> Res<Vec<u8>> {
        self.tok.clear();
        loop {
            let c = self.cur();
            check!(c != 0, "Truncated file");
            self.xml_advance();
            self.tok.push(c);
            if self.tok.ends_with(suffix) {
                break;
            }
        }
        Ok(self.tok[..self.tok.len() - suffix.len()].to_vec())
    }

    fn xml_read_until(&mut self, ctypes: u8) -> Res<Vec<u8>> {
        self.tok.clear();
        loop {
            let mut c = self.cur();
            if c == b'&' {
                let entity_begin = self.tok.len();
                loop {
                    self.xml_advance();
                    c = self.cur();
                    check!(c != 0);
                    if c == b';' {
                        break;
                    }
                    self.tok.push(c);
                }
                self.xml_advance();
                let entity = self.tok.split_off(entity_begin);

                if entity.first() == Some(&b'#') {
                    let code = if entity.get(1) == Some(&b'x') { parse_uint32_radix(&entity[2..], 16) } else { parse_uint32_radix(&entity[1..], 10) };
                    let mut bytes = [0u8; 4];
                    if code < 0x80 {
                        bytes[0] = code as u8;
                    } else if code < 0x800 {
                        bytes[0] = (0xc0 | (code >> 6)) as u8;
                        bytes[1] = (0x80 | (code & 0x3f)) as u8;
                    } else if code < 0x10000 {
                        bytes[0] = (0xe0 | (code >> 12)) as u8;
                        bytes[1] = (0x80 | ((code >> 6) & 0x3f)) as u8;
                        bytes[2] = (0x80 | (code & 0x3f)) as u8;
                    } else {
                        bytes[0] = (0xf0 | (code >> 18)) as u8;
                        bytes[1] = (0x80 | ((code >> 12) & 0x3f)) as u8;
                        bytes[2] = (0x80 | ((code >> 6) & 0x3f)) as u8;
                        bytes[3] = (0x80 | (code & 0x3f)) as u8;
                    }
                    self.tok.extend(bytes.iter().take_while(|&&b| b != 0));
                } else {
                    let ch = match &entity[..] {
                        b"lt" => b'<',
                        b"quot" => b'"',
                        b"amp" => b'&',
                        b"apos" => b'\'',
                        b"gt" => b'>',
                        _ => 0,
                    };
                    if ch != 0 {
                        self.tok.push(ch);
                    }
                }
            } else {
                if ctype(c) & ctypes != 0 {
                    break;
                }
                check!(c != 0, "Truncated file");
                self.tok.push(c);
                self.xml_advance();
            }
        }
        Ok(self.tok.clone())
    }

    /// True at a closing tag or the end of the file
    fn xml_parse_tag(&mut self, depth: usize, opening: Option<&[u8]>) -> Res<bool> {
        check!(depth < MAX_XML_DEPTH);

        if !self.xml_accept(b'<') {
            if self.cur() == 0 {
                return Ok(true);
            }
            let text = self.xml_read_until(CTYPE_TAG_START | CTYPE_END_OF_FILE)?;
            // ufbx's whitespace test also sees the token's terminating NUL, so whitespace-only text is kept too
            self.tmp_stack.push(XmlTag { text, ..Default::default() });
            return Ok(false);
        }

        if self.xml_accept(b'/') {
            let name = self.xml_read_until(CTYPE_NAME_END)?;
            check!(opening == Some(&name[..]));
            self.xml_skip_while(CTYPE_WHITESPACE);
            if !self.xml_accept(b'>') {
                return fail();
            }
            return Ok(true);
        } else if self.xml_accept(b'!') {
            if self.xml_accept(b'[') {
                for &ch in b"CDATA[" {
                    if !self.xml_accept(ch) {
                        return fail();
                    }
                }
                let text = self.xml_skip_until_string(b"]]>")?;
                self.tmp_stack.push(XmlTag { text, ..Default::default() });
            } else if self.xml_accept(b'-') {
                if !self.xml_accept(b'-') {
                    return fail();
                }
                self.xml_skip_until_string(b"-->")?;
            } else {
                self.xml_skip_until_string(b">")?;
            }
            return Ok(false);
        } else if self.xml_accept(b'?') {
            self.xml_skip_until_string(b"?>")?;
            return Ok(false);
        }

        let mut tag = XmlTag { name: self.xml_read_until(CTYPE_NAME_END)?, ..Default::default() };

        let mut has_children = false;
        loop {
            self.xml_skip_while(CTYPE_WHITESPACE);
            if self.xml_accept(b'/') {
                if !self.xml_accept(b'>') {
                    return fail();
                }
                break;
            } else if self.xml_accept(b'>') {
                has_children = true;
                break;
            } else {
                let name = self.xml_read_until(CTYPE_NAME_END)?;
                self.xml_skip_while(CTYPE_WHITESPACE);
                if !self.xml_accept(b'=') {
                    return fail();
                }
                self.xml_skip_while(CTYPE_WHITESPACE);
                let quote_ctype = if self.xml_accept(b'"') {
                    CTYPE_DOUBLE_QUOTE
                } else if self.xml_accept(b'\'') {
                    CTYPE_SINGLE_QUOTE
                } else {
                    // "Bad attrib value" is only on ufbx's error stack, not its description
                    return fail();
                };
                let value = self.xml_read_until(quote_ctype)?;
                self.xml_advance();
                tag.attribs.push(XmlAttrib { name, value });
            }
        }

        if has_children {
            let children_begin = self.tmp_stack.len();
            while !self.xml_parse_tag(depth + 1, Some(&tag.name))? {}
            tag.children = self.tmp_stack.split_off(children_begin);
        }

        self.tmp_stack.push(tag);
        Ok(false)
    }
}

/// The document's root, a nameless tag holding the top-level tags and text
pub(crate) fn load_xml(data: &[u8]) -> Res<XmlTag> {
    let mut xc = XmlContext { data, pos: 0, tok: Vec::new(), tmp_stack: Vec::new() };
    // `ufbxi_xml_parse_root` in ufbx.c
    while !xc.xml_parse_tag(0, None)? {}
    Ok(XmlTag { children: xc.tmp_stack, ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(name: &str) -> XmlTag {
        XmlTag { name: name.as_bytes().to_vec(), ..Default::default() }
    }

    fn text(t: &[u8]) -> XmlTag {
        XmlTag { text: t.to_vec(), ..Default::default() }
    }

    #[test]
    fn nested_tags_keep_their_attributes_and_the_text_between_them() {
        let root = load_xml(b"<?xml version=\"1.0\"?>\n<a x=\"1\" y = 'two'>\n  <b/>t<c></c>\n</a>").unwrap();
        let mut a = tag("a");
        a.attribs = vec![XmlAttrib { name: b"x".to_vec(), value: b"1".to_vec() }, XmlAttrib { name: b"y".to_vec(), value: b"two".to_vec() }];
        a.children = vec![text(b"\n  "), tag("b"), text(b"t"), tag("c"), text(b"\n")];
        assert_eq!(root.children, vec![text(b"\n"), a]);
        assert_eq!(root.xml_find_child("a").unwrap().xml_find_attrib("y").unwrap().value, b"two");
        assert!(root.xml_find_child("b").is_none());
    }

    #[test]
    fn entities_cdata_and_comments_are_decoded_as_ufbx_does() {
        let root = load_xml("<e>&lt;&gt;&amp;&quot;&apos;&#0;&#x0;&unknown;&#97;&#x3b2;&#12459;&#x1F602;&#X41;</e>".as_bytes()).unwrap();
        assert_eq!(root.children[0].children, vec![text("<>&\"'aβカ😂".as_bytes())]);

        let root = load_xml(b"<e><![CDATA[a<b>&lt;]]]]><!-- c --><!DOCTYPE x></e>").unwrap();
        assert_eq!(root.children[0].children, vec![text(b"a<b>&lt;]]")]);

        let root = load_xml(b"<e a=\"&lt;&#x62;\"/>").unwrap();
        assert_eq!(root.children[0].attribs[0].value, b"<b");
        assert_eq!(parse_uint32_radix(b"4294967297x", 10), 1);
        assert_eq!(parse_uint32_radix(b"fFg", 16), 255);
    }

    #[test]
    fn unclosed_tags_end_with_the_file_but_malformed_ones_fail() {
        let root = load_xml(b"<a><b>").unwrap();
        assert_eq!(root.children[0].children[0].name, b"b");

        assert_eq!(load_xml(b"<a></b>").unwrap_err().description, "");
        assert_eq!(load_xml(b"</a>").unwrap_err().description, "");
        assert_eq!(load_xml(b"<a x=1/>").unwrap_err().description, "");
        assert_eq!(load_xml(b"<a x/>").unwrap_err().description, "");
        assert_eq!(load_xml(b"<a/ >").unwrap_err().description, "");
        assert_eq!(load_xml(b"<![CDATX[").unwrap_err().description, "");
        assert_eq!(load_xml(b"<a x=\"1").unwrap_err().description, "Truncated file");
        assert_eq!(load_xml(b"<a").unwrap_err().description, "Truncated file");
        assert_eq!(load_xml(b"<!-- x").unwrap_err().description, "Truncated file");
        assert_eq!(load_xml(b"<a>&lt").unwrap_err().description, "");
        assert_eq!(load_xml(b"<a>x\0<b>").unwrap().children[0].children, vec![text(b"x")]);

        let deep = "<a>".repeat(31);
        assert!(load_xml(deep.as_bytes()).is_ok());
        let deeper = "<a>".repeat(32);
        assert!(load_xml(deeper.as_bytes()).is_err());
    }
}

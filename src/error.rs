use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ErrorType {
    #[default]
    None,
    Unknown,
    FileNotFound,
    EmptyFile,
    ExternalFileNotFound,
    OutOfMemory,
    MemoryLimit,
    AllocationLimit,
    TruncatedFile,
    Io,
    Cancelled,
    UnrecognizedFileFormat,
    UninitializedOptions,
    ZeroVertexSize,
    TruncatedVertexStream,
    InvalidUtf8,
    FeatureDisabled,
    BadNurbs,
    BadIndex,
    NodeDepthLimit,
    ThreadedAsciiParse,
    UnsafeOptions,
    DuplicateOverride,
    UnsupportedVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorType,
    pub description: String,
    pub info: String,
}

impl Error {
    pub fn new(description: impl Into<String>) -> Error {
        let description = description.into();
        Error { kind: kind_of(&description), description, info: String::new() }
    }

    /// At most 255 bytes, as `ufbxi_set_err_info` copies; a character cut there becomes '?'
    pub(crate) fn set_err_info(mut self, info: impl Into<String>) -> Error {
        let info = info.into();
        self.info = clean_string_utf8(&info.as_bytes()[..info.len().min(INFO_LENGTH - 1)]);
        self
    }

    pub(crate) fn fix_error_type(mut self, default: &str) -> Error {
        if self.description.is_empty() {
            self.description = default.to_string();
            self.kind = kind_of(default);
        }
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.info.is_empty() { f.write_str(&self.description) } else { write!(f, "{}: {}", self.description, self.info) }
    }
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.to_string()
    }
}

/// Part of `ufbxi_fix_error_type` in ufbx.c
fn kind_of(description: &str) -> ErrorType {
    match description {
        "" => ErrorType::Unknown,
        "Out of memory" => ErrorType::OutOfMemory,
        "Memory limit exceeded" => ErrorType::MemoryLimit,
        "Allocation limit exceeded" => ErrorType::AllocationLimit,
        "Truncated file" => ErrorType::TruncatedFile,
        "IO error" => ErrorType::Io,
        "Cancelled" => ErrorType::Cancelled,
        "Unrecognized file format" => ErrorType::UnrecognizedFileFormat,
        "File not found" => ErrorType::FileNotFound,
        "Empty file" => ErrorType::EmptyFile,
        "External file not found" => ErrorType::ExternalFileNotFound,
        "Uninitialized options" => ErrorType::UninitializedOptions,
        "Zero vertex size" => ErrorType::ZeroVertexSize,
        "Truncated vertex stream" => ErrorType::TruncatedVertexStream,
        "Invalid UTF-8" => ErrorType::InvalidUtf8,
        "Feature disabled" => ErrorType::FeatureDisabled,
        "Bad NURBS geometry" => ErrorType::BadNurbs,
        "Bad index" => ErrorType::BadIndex,
        "Node depth limit exceeded" => ErrorType::NodeDepthLimit,
        "Threaded ASCII parse error" => ErrorType::ThreadedAsciiParse,
        "Unsafe options" => ErrorType::UnsafeOptions,
        "Duplicate override" => ErrorType::DuplicateOverride,
        "Unsupported version" => ErrorType::UnsupportedVersion,
        _ => ErrorType::Unknown,
    }
}

pub(crate) type Res<T> = Result<T, Error>;

const INFO_LENGTH: usize = 256;

/// With `ufbxi_fail_imp_no_stack` and `ufbxi_fail_imp_err_no_stack` in ufbx.c: a failed check, which ufbx reports without a description of its own
pub(crate) fn fail<T>() -> Res<T> {
    Err(Error { kind: ErrorType::Unknown, description: String::new(), info: String::new() })
}

/// With `ufbxi_fail_imp` and `ufbxi_fail_imp_err` in ufbx.c
pub(crate) fn fail_msg<T>(msg: &str) -> Res<T> {
    Err(Error::new(msg))
}

macro_rules! check {
    ($cond:expr) => {
        if !($cond) {
            return $crate::error::fail();
        }
    };
    ($cond:expr, $msg:expr) => {
        if !($cond) {
            return $crate::error::fail_msg($msg);
        }
    };
}
pub(crate) use check;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WarningType {
    MissingExternalFile,
    ImplicitMtl,
    TruncatedArray,
    MissingGeometryData,
    DuplicateConnection,
    BadVertexWAttribute,
    MissingPolygonMapping,
    UnsupportedVersion,
    IndexClamped,
    BadUnicode,
    BadBase64Content,
    BadElementConnectedToRoot,
    DuplicateObjectId,
    EmptyFaceRemoved,
    UnknownObjDirective,
}

impl WarningType {
    pub const COUNT: usize = 15;
    pub const ALL: [WarningType; 15] = [
        WarningType::MissingExternalFile,
        WarningType::ImplicitMtl,
        WarningType::TruncatedArray,
        WarningType::MissingGeometryData,
        WarningType::DuplicateConnection,
        WarningType::BadVertexWAttribute,
        WarningType::MissingPolygonMapping,
        WarningType::UnsupportedVersion,
        WarningType::IndexClamped,
        WarningType::BadUnicode,
        WarningType::BadBase64Content,
        WarningType::BadElementConnectedToRoot,
        WarningType::DuplicateObjectId,
        WarningType::EmptyFaceRemoved,
        WarningType::UnknownObjDirective,
    ];

    /// Warnings from `IndexClamped` on are counted in place rather than repeated
    fn deduplicated(self) -> bool {
        self >= WarningType::IndexClamped
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub kind: WarningType,
    pub description: String,
    /// `NO_INDEX` if none
    pub element_id: u32,
    pub count: usize,
}

pub const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, Default)]
pub(crate) struct Warnings {
    pub list: Vec<Warning>,
    prev: [[Option<usize>; 2]; WarningType::COUNT],
    /// Index into the context's deferred element ids plus one, tagged into element ids with the high bit
    pub deferred_element_id_plus_one: u32,
}

impl Warnings {
    /// `ufbxi_warnf_imp` and `ufbxi_vwarnf_imp` in ufbx.c
    pub fn warn(&mut self, kind: WarningType, element_id: u32, description: impl Into<Vec<u8>>) {
        let mut element_id = element_id;
        if element_id == NO_INDEX && self.deferred_element_id_plus_one > 0 {
            element_id = (self.deferred_element_id_plus_one - 1) | 0x8000_0000;
        }
        let has_element = (element_id != NO_INDEX) as usize;
        if kind.deduplicated()
            && let Some(prev) = self.prev[kind as usize][has_element]
            && self.list[prev].element_id == element_id
        {
            self.list[prev].count += 1;
            return;
        }
        // ufbx prints into 256 bytes, then cleans what is no UTF-8 there, a character cut short included
        let mut description: Vec<u8> = description.into();
        description.truncate(255);
        self.list.push(Warning { kind, description: clean_string_utf8(&description), element_id, count: 1 });
        self.prev[kind as usize][has_element] = Some(self.list.len() - 1);
    }
}

/// A NUL counts as invalid
/// `utf8_valid_length(s) == s.len()`, by Rust's own check: ufbx's UTF-8 is Rust's without NUL
#[cfg(feature = "format-fbx")]
pub(crate) fn is_utf8_valid(s: &[u8]) -> bool {
    std::str::from_utf8(s).is_ok() && !s.contains(&0)
}

pub(crate) fn utf8_valid_length(s: &[u8]) -> usize {
    let mut index = 0;
    while index < s.len() {
        let c = s[index];
        let left = s.len() - index;
        if c & 0x80 == 0 {
            if c != 0 {
                index += 1;
                continue;
            }
        } else if c & 0xe0 == 0xc0 && left >= 2 {
            let code = (c as u32) << 8 | s[index + 1] as u32;
            if code & 0xc0 == 0x80 && code >= 0xc280 {
                index += 2;
                continue;
            }
        } else if c & 0xf0 == 0xe0 && left >= 3 {
            let code = (c as u32) << 16 | (s[index + 1] as u32) << 8 | s[index + 2] as u32;
            if code & 0xc0c0 == 0x8080 && code >= 0xe0a080 && !(0xeda080..0xee8080).contains(&code) {
                index += 3;
                continue;
            }
        } else if c & 0xf8 == 0xf0 && left >= 4 {
            let code = (c as u32) << 24 | (s[index + 1] as u32) << 16 | (s[index + 2] as u32) << 8 | s[index + 3] as u32;
            if code & 0xc0c0c0 == 0x808080 && (0xf0908080..=0xf48fbfbf).contains(&code) {
                index += 4;
                continue;
            }
        }
        break;
    }
    index
}

/// Every invalid byte becomes '?'
pub(crate) fn clean_string_utf8(s: &[u8]) -> String {
    let mut out = s.to_vec();
    let mut pos = 0;
    loop {
        pos += utf8_valid_length(&out[pos..]);
        if pos == out.len() {
            break;
        }
        out[pos] = b'?';
        pos += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_errors_info_is_cut_where_ufbx_cuts_it() {
        let long = Error::new("File not found").set_err_info("a".repeat(300));
        assert_eq!(long.info.len(), 255);
        let cut = Error::new("File not found").set_err_info(format!("{}é", "a".repeat(254)));
        assert_eq!(cut.info, format!("{}?", "a".repeat(254)));
    }
}

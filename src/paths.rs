use crate::types::*;

pub(crate) const DEFAULT_PATH_SEPARATOR: u8 = if cfg!(windows) { b'\\' } else { b'/' };

pub(crate) fn trim_delimiters(path_separator: u8, data: &[u8]) -> usize {
    let mut length = data.len();
    while length > 0 {
        let c = data[length - 1];
        if c == b'/' || c == path_separator {
            length -= 1;
            break;
        }
        length -= 1;
    }
    length
}

pub(crate) fn init_file_paths(metadata: &mut Metadata, opts: &LoadOpts) {
    if !opts.filename.is_empty() {
        metadata.filename = opts.filename.clone();
    } else if !opts.raw_filename.is_empty() {
        metadata.filename = crate::error::clean_string_utf8(&opts.raw_filename);
    }
    if !opts.raw_filename.is_empty() {
        metadata.raw_filename = opts.raw_filename.clone();
    } else if !opts.filename.is_empty() {
        metadata.raw_filename = opts.filename.as_bytes().to_vec();
    }
    let n = trim_delimiters(opts.path_separator, metadata.filename.as_bytes());
    metadata.relative_root = String::from_utf8_lossy(&metadata.filename.as_bytes()[..n]).into_owned();
    let n = trim_delimiters(opts.path_separator, &metadata.raw_filename);
    metadata.raw_relative_root = metadata.raw_filename[..n].to_vec();
}

pub(crate) fn is_absolute_path(path: &[u8]) -> bool {
    (!path.is_empty() && (path[0] == b'/' || path[0] == b'\\')) || (path.len() > 2 && path[1] == b':' && (path[2] == b'\\' || path[2] == b'/'))
}

/// Each `../` cancels a directory
pub(crate) fn resolve_relative_filename(path_separator: u8, relative_root: &[u8], src: &[u8]) -> Vec<u8> {
    let mut src = src;
    while !src.is_empty() && (src[0] == b'/' || src[0] == b'\\') {
        src = &src[1..];
    }
    if src.is_empty() {
        return Vec::new();
    }

    let prefix = relative_root;
    let mut prefix_length = prefix.len();
    if is_absolute_path(src) {
        prefix_length = 0;
    }

    while prefix_length > 0 && src.len() >= 3 && src[0] == b'.' && src[1] == b'.' && (src[2] == b'/' || src[2] == b'\\') {
        let mut part_start = prefix_length;
        while part_start > 0 && !(prefix[part_start - 1] == b'/' || prefix[part_start - 1] == b'\\') {
            part_start -= 1;
        }
        let part_len = prefix_length - part_start;
        if part_len == 2 && prefix[part_start] == b'.' && prefix[part_start + 1] == b'.' {
            break;
        }
        prefix_length = if part_start > 0 { part_start - 1 } else { 0 };
        if part_len == 1 && prefix[part_start] == b'.' {
            continue;
        }
        src = &src[3..];
    }

    let mut result = Vec::with_capacity(prefix_length + src.len() + 1);
    if prefix_length > 0 {
        result.extend_from_slice(&prefix[..prefix_length]);
        result.push(path_separator);
    }
    for &c in src {
        result.push(if c == b'/' || c == b'\\' { path_separator } else { c });
    }
    result
}

#[cfg_attr(not(any(feature = "format-obj", feature = "geometry-cache")), allow(dead_code))]
pub(crate) fn open_file(cb: Option<&std::sync::Arc<OpenFileFn>>, path: &str, original_filename: Option<&[u8]>, file_type: OpenFileType) -> Option<Vec<u8>> {
    let info = OpenFileInfo { file_type, original_filename: original_filename.map_or_else(|| path.as_bytes().to_vec(), |o| o.to_vec()) };
    match cb {
        Some(cb) => cb(path, &info),
        None => default_open_file(path, &info),
    }
}

pub fn default_open_file(path: &str, _info: &OpenFileInfo) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WarningType;
    use crate::load::load_memory;

    #[test]
    fn a_relative_name_joins_the_root_and_each_dot_dot_takes_a_directory_off() {
        assert_eq!(resolve_relative_filename(b'/', b"/a/b/c", b"../../t.png"), b"/a/t.png");
        assert_eq!(resolve_relative_filename(b'/', b"/a/b/./c", b"../../t.png"), b"/a/t.png");
        assert_eq!(resolve_relative_filename(b'/', b"/a/..", b"../t.png"), b"/a/../../t.png");
        assert_eq!(resolve_relative_filename(b'/', b"/a/b", b"C:\\x\\t.png"), b"C:/x/t.png");
        assert_eq!(resolve_relative_filename(b'/', b"/a/b", b"\\\\"), b"");
        assert_eq!(trim_delimiters(b'/', b"/a/b/model.fbx"), 4);
        assert_eq!(trim_delimiters(b'/', b"model.fbx"), 0);
    }

    const MINIMAL: &[u8] = b"; FBX 7.4.0 project file\nObjects:  {\n}\n";

    #[cfg(feature = "format-fbx")]
    #[test]
    fn a_file_name_is_cleaned_as_ufbx_cleans_its_strings() {
        let named = load_memory(MINIMAL, &LoadOpts { filename: "path\0/file.fbx".into(), ..Default::default() }).unwrap();
        // ToDo: expected: "path\u{fffd}/file.fbx", "path\u{fffd}"
        assert_eq!((named.metadata.filename.as_str(), named.metadata.relative_root.as_str()), ("path\0/file.fbx", "path\0"));
        // ToDo: expected: true
        assert!(!named.metadata.has_warning[WarningType::BadUnicode as usize]);
        assert_eq!(named.metadata.raw_filename, b"path\0/file.fbx");

        let raw = load_memory(MINIMAL, &LoadOpts { raw_filename: b"path\0/file.fbx".to_vec(), ..Default::default() }).unwrap();
        // ToDo: expected: "path\u{fffd}/file.fbx"
        assert_eq!(raw.metadata.filename, "path?/file.fbx");

        let strict = LoadOpts { filename: "path\0/file.fbx".into(), unicode_error_handling: UnicodeErrorHandling::AbortLoading, ..Default::default() };
        // ToDo: expected: the load refused, "Invalid UTF-8"
        assert!(load_memory(MINIMAL, &strict).is_ok());
    }
}

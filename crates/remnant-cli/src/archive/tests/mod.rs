mod setup;

use super::*;
use crate::archive::ArchiveError;
use setup::*;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[test]
fn rejects_non_gzip_tgz() {
    let path = test_path("not-gzip.tgz");
    remove_path_if_exists(&path);

    let mut file = File::create(&path).expect("test file should be created");
    file.write_all(b"this is not a gzip stream")
        .expect("test file should be written");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveReadFailed {
            path,
            kind: io::ErrorKind::InvalidInput,
        })
    );
}

#[test]
fn rejects_empty_archive() {
    let path = test_path("empty.tgz");

    create_empty_tgz(&path);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(result, Err(ArchiveError::ArchiveIsEmpty(path)));
}

#[test]
fn rejects_gzip_that_is_not_tar_archive() {
    let path = test_path("not-tar.tgz");

    create_gzip_with_bytes(&path, b"this is gzip data, but not a tar archive");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    match result {
        Err(ArchiveError::ArchiveReadFailed {
            path: result_path, ..
        }) => {
            assert_eq!(result_path, path);
        }
        other => panic!("expected archive read failure, got {other:?}"),
    }
}

#[test]
fn committed_non_gzip_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("non-gzip-tgz");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    match result {
        Err(ArchiveError::ArchiveReadFailed {
            path: result_path, ..
        }) => assert_eq!(result_path, path),
        other => panic!("expected archive read failure, got {other:?}"),
    }
}

#[test]
fn committed_gzip_not_tar_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("gzip-not-tar");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    match result {
        Err(ArchiveError::ArchiveReadFailed {
            path: result_path, ..
        }) => assert_eq!(result_path, path),
        other => panic!("expected archive read failure, got {other:?}"),
    }
}

#[test]
fn committed_empty_archive_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("empty-archive");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    assert_eq!(result, Err(ArchiveError::ArchiveIsEmpty(path)));
}

#[test]
fn committed_missing_package_json_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("missing-package-json");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    assert_eq!(result, Err(ArchiveError::PackageJsonMissing(path)));
}

#[test]
fn committed_duplicate_archive_entry_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("duplicate-archive-entry");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathDuplicate(PathBuf::from(
            "package/package.json"
        )))
    );
}

#[test]
fn committed_path_traversal_archive_entry_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("path-traversal-archive-entry");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "package/../package.json"
        )))
    );
}

#[test]
fn committed_unsupported_directory_entry_fixture_is_rejected() {
    let path = malformed_fixture_artifact_path("unsupported-directory-entry");

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTypeUnsupported {
            path: PathBuf::from("package/lib"),
            entry_type: b'5',
        })
    );
}

#[test]
fn accepts_safe_archive_entry() {
    let path = test_path("safe.tgz");

    create_tgz_with_file(&path, "package/package.json", br#"{"name":"safe"}"#);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Ok(vec![ArchiveEntry {
            path: PathBuf::from("package/package.json"),
            size: br#"{"name":"safe"}"#.len() as u64,
        }])
    );
}

#[test]
fn archive_errors_escape_terminal_control_characters() {
    let error = ArchiveError::ArchiveEntryPathDuplicate(PathBuf::from("package/file\nname.js"));

    assert_eq!(
        error.to_string(),
        r"archive entry path is duplicated: package/file\nname.js"
    );
}

#[test]
fn normalizes_current_dir_archive_entry_path_components() {
    let path = test_path("current-dir-components.tgz");

    create_tgz_with_raw_file_path(
        &path,
        "./package/./package.json",
        br#"{"name":"normalized"}"#,
    );

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Ok(vec![ArchiveEntry {
            path: PathBuf::from("package/package.json"),
            size: br#"{"name":"normalized"}"#.len() as u64,
        }])
    );
}

#[test]
fn rejects_archive_entry_path_that_normalizes_to_empty() {
    let path = test_path("empty-normalized-entry-path.tgz");

    create_tgz_with_raw_file_path(&path, "./", b"unsafe");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from("./")))
    );
}

#[test]
fn rejects_duplicate_archive_entry_path_after_normalization() {
    let path = test_path("duplicate-normalized-entry.tgz");

    create_tgz_with_raw_file_entries(
        &path,
        &[
            ("package/package.json", br#"{"name":"first"}"#.as_ref()),
            ("package/./package.json", br#"{"name":"second"}"#.as_ref()),
        ],
    );

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathDuplicate(PathBuf::from(
            "package/package.json"
        )))
    );
}

#[test]
fn rejects_archive_with_too_many_entries() {
    let path = test_path("too-many-entries.tgz");

    create_tgz_with_file_count(&path, MAX_ARCHIVE_ENTRIES + 1);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveTooManyEntries {
            path,
            count: MAX_ARCHIVE_ENTRIES + 1,
            limit: MAX_ARCHIVE_ENTRIES,
        })
    );
}

#[test]
fn rejects_archive_entry_path_that_is_too_long() {
    let path = test_path("long-entry-path.tgz");
    let archive_path = format!(
        "package/{}",
        "a".repeat(MAX_ARCHIVE_ENTRY_PATH_BYTES + 1 - "package/".len())
    );

    create_tgz_with_file(&path, &archive_path, b"");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathTooLong {
            length: MAX_ARCHIVE_ENTRY_PATH_BYTES + 1,
            limit: MAX_ARCHIVE_ENTRY_PATH_BYTES,
        })
    );
}

#[test]
fn rejects_archive_entry_that_is_too_large() {
    let path = test_path("large-entry.tgz");

    create_tgz_with_raw_declared_file_size(&path, "package/large.js", MAX_ARCHIVE_ENTRY_BYTES + 1);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTooLarge {
            path: PathBuf::from("package/large.js"),
            size: MAX_ARCHIVE_ENTRY_BYTES + 1,
            limit: MAX_ARCHIVE_ENTRY_BYTES,
        })
    );
}

#[test]
fn accepts_decompressed_archive_stream_at_limit() {
    let mut reader = DecompressedArchiveReader::new(io::Cursor::new(b"abc"), 3);
    let mut contents = Vec::new();

    let result = reader.read_to_end(&mut contents);

    assert_eq!(result.expect("reader should accept data at limit"), 3);
    assert_eq!(contents, b"abc");
}

#[test]
fn rejects_decompressed_archive_stream_over_limit() {
    let path = test_path("decompressed-over-limit.tgz");
    let mut reader = DecompressedArchiveReader::new(io::Cursor::new(b"abc"), 2);
    let mut contents = Vec::new();

    let error = reader
        .read_to_end(&mut contents)
        .expect_err("reader should reject decompressed data past limit");
    let result = map_archive_read_error(&path, error);

    assert_eq!(contents, b"ab");
    assert_eq!(
        result,
        ArchiveError::ArchiveDecompressedTooLarge { path, limit: 2 }
    );
}

#[test]
fn accepts_archive_total_size_at_limit() {
    let path = test_path("total-size-at-limit.tgz");
    let mut total_size = MAX_ARCHIVE_TOTAL_BYTES - 1;

    let result = add_archive_entry_size(&path, &mut total_size, 1);

    assert_eq!(result, Ok(()));
    assert_eq!(total_size, MAX_ARCHIVE_TOTAL_BYTES);
}

#[test]
fn rejects_archive_total_size_over_limit() {
    let path = test_path("total-size-over-limit.tgz");
    let mut total_size = MAX_ARCHIVE_TOTAL_BYTES;

    let result = add_archive_entry_size(&path, &mut total_size, 1);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveTooLarge {
            path,
            size: MAX_ARCHIVE_TOTAL_BYTES + 1,
            limit: MAX_ARCHIVE_TOTAL_BYTES,
        })
    );
}

#[test]
fn rejects_archive_total_size_overflow() {
    let path = test_path("total-size-overflow.tgz");
    let mut total_size = u64::MAX;

    let result = add_archive_entry_size(&path, &mut total_size, 1);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveTooLarge {
            path,
            size: u64::MAX,
            limit: MAX_ARCHIVE_TOTAL_BYTES,
        })
    );
}

#[test]
fn rejects_absolute_archive_entry_path() {
    let path = test_path("absolute-entry.tgz");

    create_tgz_with_raw_file_path(&path, "/package/package.json", br#"{"name":"unsafe"}"#);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "/package/package.json"
        )))
    );
}

#[test]
fn rejects_parent_directory_traversal_archive_entry_path() {
    let path = test_path("parent-traversal-entry.tgz");

    create_tgz_with_raw_file_path(&path, "package/../package.json", br#"{"name":"unsafe"}"#);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "package/../package.json"
        )))
    );
}

#[test]
fn rejects_windows_backslash_archive_entry_path() {
    let path = test_path("windows-backslash-entry.tgz");

    create_tgz_with_raw_file_path(&path, r"package\..\package.json", br#"{"name":"unsafe"}"#);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            r"package\..\package.json"
        )))
    );
}

#[test]
fn rejects_windows_drive_prefix_archive_entry_path() {
    let path = test_path("windows-drive-entry.tgz");

    create_tgz_with_raw_file_path(&path, "C:/package/package.json", br#"{"name":"unsafe"}"#);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "C:/package/package.json"
        )))
    );
}

#[test]
fn rejects_duplicate_archive_entry_path() {
    let path = test_path("duplicate-entry.tgz");

    create_tgz_with_duplicate_file_path(&path);

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathDuplicate(PathBuf::from(
            "package/package.json"
        )))
    );
}

#[test]
fn rejects_symlink_archive_entry() {
    let path = test_path("symlink-entry.tgz");

    create_tgz_with_symlink(&path, "package/link", "package/package.json");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryIsSymlink(PathBuf::from(
            "package/link"
        )))
    );
}

#[test]
fn rejects_hardlink_archive_entry() {
    let path = test_path("hardlink-entry.tgz");

    create_tgz_with_hardlink(&path, "package/link", "package/package.json");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryIsHardlink(PathBuf::from(
            "package/link"
        )))
    );
}

#[test]
fn rejects_directory_archive_entry() {
    let path = test_path("directory-entry.tgz");

    create_tgz_with_directory(&path, "package/lib");

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTypeUnsupported {
            path: PathBuf::from("package/lib"),
            entry_type: b'5',
        })
    );
}

#[test]
fn rejects_unsupported_archive_entry_type() {
    let path = test_path("unsupported-entry-type.tgz");

    create_tgz_with_raw_entry_type(&path, "package/fifo", b'6');

    let result = read_archive_entries(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTypeUnsupported {
            path: PathBuf::from("package/fifo"),
            entry_type: b'6',
        })
    );
}

#[test]
fn inspects_archive_entries_and_package_json_bytes() {
    let path = test_path("archive-inspection.tgz");
    let package_json = br#"{"name":"metadata"}"#;
    let index_js = b"console.log('hello');";

    create_tgz_with_raw_file_entries(
        &path,
        &[
            ("package/index.js", index_js.as_ref()),
            ("package/package.json", package_json.as_ref()),
        ],
    );

    let result = inspect_archive(
        File::open(&path).expect("fixture file should be openable"),
        &path,
    );

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Ok(ArchiveInspection {
            entries: vec![
                ArchiveEntry {
                    path: PathBuf::from("package/index.js"),
                    size: index_js.len() as u64,
                },
                ArchiveEntry {
                    path: PathBuf::from("package/package.json"),
                    size: package_json.len() as u64,
                },
            ],
            package_json: package_json.to_vec(),
        })
    );
}

#[test]
fn reads_package_json_bytes() {
    let path = test_path("package-json.tgz");
    let contents = br#"{"name":"metadata"}"#;

    create_tgz_with_file(&path, "package/package.json", contents);

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(result, Ok(contents.to_vec()));
}

#[test]
fn rejects_missing_package_json() {
    let path = test_path("missing-package-json.tgz");

    create_tgz_with_file(&path, "package/index.js", b"console.log('hello');");

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(result, Err(ArchiveError::PackageJsonMissing(path)));
}

#[test]
fn rejects_oversized_package_json() {
    let path = test_path("oversized-package-json.tgz");
    let contents = vec![b'a'; (MAX_PACKAGE_JSON_BYTES + 1) as usize];

    create_tgz_with_file(&path, "package/package.json", &contents);

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::PackageJsonTooLarge {
            path: PathBuf::from("package/package.json"),
            size: MAX_PACKAGE_JSON_BYTES + 1,
            limit: MAX_PACKAGE_JSON_BYTES,
        })
    );
}

#[test]
fn rejects_package_json_when_entry_is_directory() {
    let path = test_path("package-json-directory.tgz");

    create_tgz_with_directory(&path, "package/package.json");

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTypeUnsupported {
            path: PathBuf::from("package/package.json"),
            entry_type: b'5',
        })
    );
}

#[test]
fn read_package_json_rejects_unsafe_entries_in_same_archive() {
    let path = test_path("package-json-with-unsafe-entry.tgz");

    create_tgz_with_raw_file_entries(
        &path,
        &[
            ("package/package.json", br#"{"name":"safe"}"#.as_ref()),
            ("/package/evil.js", b"unsafe".as_ref()),
        ],
    );

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "/package/evil.js"
        )))
    );
}

#[test]
fn read_package_json_rejects_oversized_entries_in_same_archive() {
    let path = test_path("package-json-with-large-entry.tgz");

    create_tgz_with_package_json_and_raw_declared_file_size(
        &path,
        "package/large.js",
        MAX_ARCHIVE_ENTRY_BYTES + 1,
    );

    let result = read_package_json(&path);

    remove_path_if_exists(&path);

    assert_eq!(
        result,
        Err(ArchiveError::ArchiveEntryTooLarge {
            path: PathBuf::from("package/large.js"),
            size: MAX_ARCHIVE_ENTRY_BYTES + 1,
            limit: MAX_ARCHIVE_ENTRY_BYTES,
        })
    );
}

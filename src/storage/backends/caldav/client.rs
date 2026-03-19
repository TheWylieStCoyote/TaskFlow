//! CalDAV HTTP client operations.
//!
//! Implements the CalDAV/WebDAV HTTP methods needed for VTODO synchronisation:
//! - `REPORT` to list all VTODOs in a collection
//! - `PUT` to create or update a VTODO
//! - `DELETE` to remove a VTODO

use crate::domain::Task;
#[cfg(feature = "caldav-sync")]
use crate::storage::export_to_ics;
use crate::storage::{import_from_ics, ImportOptions, StorageError, StorageResult};

use super::CalDavConfig;

/// REPORT request body for querying all VTODO components.
#[cfg(feature = "caldav-sync")]
const CALENDAR_QUERY_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data/>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VTODO"/>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#;

/// Fetch all VTODOs from the CalDAV collection.
///
/// Issues a `REPORT` request and parses each embedded `VCALENDAR` block.
///
/// # Errors
///
/// Returns [`StorageError`] on HTTP errors or parse failures.
#[cfg(feature = "caldav-sync")]
pub fn fetch_all_vtodos(config: &CalDavConfig) -> StorageResult<Vec<Task>> {
    let client = build_client(config)?;
    let url = format!(
        "{}{}",
        config.url.trim_end_matches('/'),
        config.collection_path
    );

    let response = client
        .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Depth", "1")
        .body(CALENDAR_QUERY_XML)
        .send()
        .map_err(|e| StorageError::serialization(format!("CalDAV REPORT failed: {e}")))?;

    if !response.status().is_success() {
        return Err(StorageError::serialization(format!(
            "CalDAV REPORT returned HTTP {}",
            response.status()
        )));
    }

    let body = response
        .text()
        .map_err(|e| StorageError::serialization(format!("Failed to read CalDAV response: {e}")))?;

    Ok(parse_vtodos_from_report(&body))
}

/// Push (create or update) a task to the CalDAV collection as a VTODO.
///
/// Uses a `PUT` request to `{collection}/{uid}.ics`.
///
/// # Errors
///
/// Returns [`StorageError`] on HTTP errors or serialisation failures.
#[cfg(feature = "caldav-sync")]
pub fn push_vtodo(config: &CalDavConfig, task: &Task) -> StorageResult<()> {
    let client = build_client(config)?;
    let url = vtodo_url(config, task);

    let mut ics_body = Vec::new();
    export_to_ics(std::slice::from_ref(task), &mut ics_body).map_err(|e| {
        StorageError::serialization(format!("Failed to serialise task as ICS: {e}"))
    })?;

    let response = client
        .put(&url)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(ics_body)
        .send()
        .map_err(|e| StorageError::serialization(format!("CalDAV PUT failed: {e}")))?;

    if !response.status().is_success()
        && response.status() != reqwest::StatusCode::CREATED
        && response.status() != reqwest::StatusCode::NO_CONTENT
    {
        return Err(StorageError::serialization(format!(
            "CalDAV PUT returned HTTP {}",
            response.status()
        )));
    }

    Ok(())
}

/// Delete a VTODO from the CalDAV collection.
///
/// # Errors
///
/// Returns [`StorageError`] on HTTP errors (404 is treated as success).
#[cfg(feature = "caldav-sync")]
pub fn delete_vtodo(config: &CalDavConfig, task_id: &crate::domain::TaskId) -> StorageResult<()> {
    let client = build_client(config)?;
    let url = format!(
        "{}{}{}.ics",
        config.url.trim_end_matches('/'),
        config.collection_path,
        task_id.0
    );

    let response = client
        .delete(&url)
        .send()
        .map_err(|e| StorageError::serialization(format!("CalDAV DELETE failed: {e}")))?;

    // 404 = already gone, treat as success
    if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(StorageError::serialization(format!(
            "CalDAV DELETE returned HTTP {}",
            response.status()
        )));
    }

    Ok(())
}

// ── No-op stubs when caldav-sync feature is disabled ─────────────────────────

/// No-op fetch when CalDAV is not compiled in.
#[cfg(not(feature = "caldav-sync"))]
pub fn fetch_all_vtodos(_config: &CalDavConfig) -> StorageResult<Vec<Task>> {
    Err(StorageError::serialization(
        "CalDAV sync requires the 'caldav-sync' cargo feature",
    ))
}

/// No-op push when CalDAV is not compiled in.
#[cfg(not(feature = "caldav-sync"))]
pub fn push_vtodo(_config: &CalDavConfig, _task: &Task) -> StorageResult<()> {
    Err(StorageError::serialization(
        "CalDAV sync requires the 'caldav-sync' cargo feature",
    ))
}

/// No-op delete when CalDAV is not compiled in.
#[cfg(not(feature = "caldav-sync"))]
pub fn delete_vtodo(_config: &CalDavConfig, _task_id: &crate::domain::TaskId) -> StorageResult<()> {
    Err(StorageError::serialization(
        "CalDAV sync requires the 'caldav-sync' cargo feature",
    ))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a blocking reqwest client with Basic auth credentials.
#[cfg(feature = "caldav-sync")]
fn build_client(config: &CalDavConfig) -> StorageResult<reqwest::blocking::Client> {
    use reqwest::header::{HeaderMap, AUTHORIZATION};

    let creds = base64_encode(&format!("{}:{}", config.username, config.password));
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Basic {creds}")
            .parse()
            .map_err(|e| StorageError::serialization(format!("Invalid auth header: {e}")))?,
    );

    reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| StorageError::serialization(format!("Failed to build HTTP client: {e}")))
}

/// Encode bytes as Base64 (RFC 4648, standard alphabet with padding).
#[cfg(feature = "caldav-sync")]
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = usize::from(chunk[0]);
        let b1 = if chunk.len() > 1 {
            usize::from(chunk[1])
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            usize::from(chunk[2])
        } else {
            0
        };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(char::from(ALPHABET[(combined >> 18) & 0x3F]));
        out.push(char::from(ALPHABET[(combined >> 12) & 0x3F]));
        out.push(if chunk.len() > 1 {
            char::from(ALPHABET[(combined >> 6) & 0x3F])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ALPHABET[combined & 0x3F])
        } else {
            '='
        });
    }
    out
}

/// Build the canonical URL for a task's `.ics` file on the CalDAV server.
#[cfg(feature = "caldav-sync")]
fn vtodo_url(config: &CalDavConfig, task: &Task) -> String {
    format!(
        "{}{}{}.ics",
        config.url.trim_end_matches('/'),
        config.collection_path,
        task.id.0
    )
}

/// Extract all VCALENDAR blocks embedded in a CalDAV REPORT XML response and
/// parse them into `Task` values using the existing ICS importer.
#[cfg_attr(not(any(feature = "caldav-sync", test)), allow(dead_code))]
fn parse_vtodos_from_report(xml_body: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut remainder = xml_body;

    while let Some(start) = remainder.find("BEGIN:VCALENDAR") {
        remainder = &remainder[start..];
        let end_marker = "END:VCALENDAR";
        let Some(end_pos) = remainder.find(end_marker) else {
            break;
        };
        let ics_block = &remainder[..end_pos + end_marker.len()];
        remainder = &remainder[end_pos + end_marker.len()..];

        // Decode XML character entities that may be present when ICS is
        // embedded inside XML.
        let decoded = decode_xml_entities(ics_block);
        let cursor = std::io::Cursor::new(decoded.as_bytes());
        let opts = ImportOptions {
            validate: false,
            ..ImportOptions::default()
        };
        match import_from_ics(cursor, &opts) {
            Ok(result) => tasks.extend(result.imported),
            Err(e) => {
                tracing::warn!("Failed to parse VTODO from CalDAV response: {e}");
            }
        }
    }

    tasks
}

/// Decode common XML character entities in ICS data embedded inside XML.
#[cfg_attr(not(any(feature = "caldav-sync", test)), allow(dead_code))]
fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#13;", "\r")
        .replace("&#10;", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vtodos_empty_body() {
        let result = parse_vtodos_from_report("<multistatus></multistatus>");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_vtodos_from_report_single() {
        let xml = r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/caldav/user/tasks/abc.ics</href>
    <propstat>
      <prop>
        <C:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:abc
SUMMARY:Buy groceries
STATUS:NEEDS-ACTION
END:VTODO
END:VCALENDAR</C:calendar-data>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#;

        let tasks = parse_vtodos_from_report(xml);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Buy groceries");
    }

    #[test]
    fn test_parse_vtodos_from_report_multiple() {
        let xml = r"<multistatus>
  <response>
    <C:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:aaa
SUMMARY:Task One
END:VTODO
END:VCALENDAR</C:calendar-data>
  </response>
  <response>
    <C:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:bbb
SUMMARY:Task Two
END:VTODO
END:VCALENDAR</C:calendar-data>
  </response>
</multistatus>";

        let tasks = parse_vtodos_from_report(xml);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Task One");
        assert_eq!(tasks[1].title, "Task Two");
    }

    #[test]
    fn test_decode_xml_entities() {
        assert_eq!(decode_xml_entities("hello &amp; world"), "hello & world");
        assert_eq!(decode_xml_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_xml_entities("&quot;quoted&quot;"), "\"quoted\"");
    }

    #[cfg(feature = "caldav-sync")]
    #[test]
    fn test_base64_encode_basic() {
        // RFC 4648 test vectors
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foob"), "Zm9vYg==");
        assert_eq!(base64_encode("fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
    }
}

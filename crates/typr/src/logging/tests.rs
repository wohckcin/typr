use super::*;

use log::Level;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::channel;

/// Run `body` with a record at `level`; the caller owns the arguments, as `log` macros do.
fn with_record(level: Level, args: std::fmt::Arguments<'_>, body: impl FnOnce(&Record<'_>)) {
    body(
        &Record::builder()
            .level(level)
            .target("typr::test")
            .args(args)
            .build(),
    );
}

fn line_of(level: Level, message: &str) -> String {
    Sink::line(
        &Record::builder()
            .level(level)
            .target("typr::test")
            .args(format_args!("{message}"))
            .build(),
    )
}

#[test]
fn a_line_carries_the_level_the_target_and_the_message() {
    let (tx, _rx) = mpsc::sync_channel(4);
    let _sink = Sink::new(tx, LevelFilter::Info);
    let line = line_of(Level::Warn, "the library stopped");
    assert!(line.contains("WARN"), "{line}");
    assert!(line.contains("typr::test"), "{line}");
    assert!(line.ends_with("the library stopped"), "{line}");
    assert!(
        line[..4].chars().all(|c| c.is_ascii_digit()) && line[4..5].eq("-"),
        "a line starts with a timestamp: {line}"
    );
}

#[test]
fn nothing_below_the_filter_is_sent() {
    let (tx, rx) = mpsc::sync_channel(4);
    let sink = Sink::new(tx, LevelFilter::Warn);
    with_record(Level::Info, format_args!("not interesting"), |record| {
        sink.log(record)
    });
    assert!(rx.try_recv().is_err(), "an info record is filtered out");
    with_record(Level::Error, format_args!("interesting"), |record| {
        sink.log(record)
    });
    assert!(matches!(rx.try_recv(), Ok(Message::Line(line)) if line.contains("interesting")));
}

#[test]
fn a_written_record_reaches_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    let (tx, rx) = mpsc::sync_channel(4);
    let writer = {
        let path = path.clone();
        std::thread::spawn(move || write_loop(rx, &path, Arc::new(AtomicU64::new(0))))
    };

    tx.send(Message::Line("hello".to_string())).expect("sent");
    let (reply_tx, reply_rx) = channel();
    tx.send(Message::Flush(reply_tx)).expect("flush");
    reply_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("flushed");
    drop(tx);
    writer.join().expect("joined");

    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("hello"), "{text}");
}

#[test]
fn a_full_channel_drops_records_and_the_next_write_counts_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    let (tx, rx) = mpsc::sync_channel(1);
    let sink = Sink::new(tx.clone(), LevelFilter::Warn);

    for message in ["first", "second", "third"] {
        with_record(Level::Warn, format_args!("{message}"), |record| {
            sink.log(record)
        });
    }
    assert_eq!(sink.dropped(), 2, "two records did not fit");

    let writer = {
        let path = path.clone();
        let counter = sink.counter();
        std::thread::spawn(move || write_loop(rx, &path, counter))
    };
    let (reply_tx, reply_rx) = channel();
    tx.send(Message::Flush(reply_tx)).expect("flush");
    reply_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("flushed");
    drop(tx);
    drop(sink);
    writer.join().expect("joined");

    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("2 records dropped"), "{text}");
    assert!(text.contains("first"), "{text}");
}

#[test]
fn an_oversized_log_is_rotated_once_on_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    let oversized = usize::try_from(SIZE_LIMIT + 1).expect("a size that fits");
    std::fs::write(&path, vec![b'x'; oversized]).expect("write");
    let rotated = dir.path().join(ROTATED_NAME);
    std::fs::write(&rotated, "older").expect("write the old copy");

    rotate(&path).expect("rotate");

    assert!(!path.exists(), "the oversized file moved aside");
    let moved = std::fs::metadata(&rotated).expect("rotated").len();
    assert_eq!(moved, SIZE_LIMIT + 1, "the newest oversized file wins");
    let text = std::fs::read_to_string(&rotated).expect("read");
    assert!(text.starts_with("xxx"), "the old copy was replaced");
}

#[test]
fn a_log_under_the_limit_is_left_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(FILE_NAME);
    std::fs::write(&path, "small").expect("write");

    rotate(&path).expect("rotate");

    assert!(path.exists(), "a small log stays put");
    assert!(!dir.path().join(ROTATED_NAME).exists());
}

#[test]
fn the_level_defaults_to_warn_and_follows_the_variable() {
    assert_eq!(level_from(None), LevelFilter::Warn);
    assert_eq!(level_from(Some("  ")), LevelFilter::Warn);
    assert_eq!(level_from(Some("nonsense")), LevelFilter::Warn);
    assert_eq!(level_from(Some("info")), LevelFilter::Info);
    assert_eq!(level_from(Some("TRACE")), LevelFilter::Trace);
}

#[test]
fn a_description_walks_the_whole_chain() {
    use typr_core::store::StoreError;

    let error = StoreError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let described = describe(&error);
    assert!(
        described.starts_with("a file operation on the library failed"),
        "{described}"
    );
    assert!(described.contains("denied"), "{described}");
    assert_eq!(
        described.matches("denied").count(),
        1,
        "the cause appears once: {described}"
    );
}

#[test]
fn installing_writes_the_next_record_and_the_quit_reaches_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = install(dir.path(), LevelFilter::Warn).expect("installed");
    log::warn!(target: "typr", "the library stopped");
    shutdown();
    let written = std::fs::read_to_string(&path).expect("read the log");
    assert!(
        written.contains("the library stopped"),
        "the record reached the file: {written}"
    );
}

#[test]
fn a_timestamp_is_readable() {
    let text = timestamp(1_768_089_600_000);
    assert_eq!(text.len(), 23, "{text}");
    assert_eq!(&text[4..5], "-");
    assert_eq!(&text[10..11], " ");
    assert_eq!(&text[19..20], ".");
}

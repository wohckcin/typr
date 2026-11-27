//! Where typr writes down what went wrong.
//!
//! Two threads, deliberately. The `log::Log` implementation runs wherever the record is made —
//! including the frame thread — so it only formats a line and hands it to a bounded channel; a
//! writer thread owns the file, the rotation and the flush. Rune forbids file work in the frame
//! loop, and the failure sites that most need a record are the ones that run in a frame.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use log::{LevelFilter, Log, Metadata, Record};

use crate::day_offset::local_offset_ms;

/// How large the log may grow before the next start rotates it.
pub const SIZE_LIMIT: u64 = 1024 * 1024;

/// The file name inside the application's logs folder, and the one rotated copy beside it.
pub const FILE_NAME: &str = "typr.log";
pub const ROTATED_NAME: &str = "typr.log.1";

/// How many lines may wait for the writer before records start being dropped.
const CHANNEL_CAPACITY: usize = 256;

/// How long a quit waits for the writer to reach the file.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

static SENDER: OnceLock<SyncSender<Message>> = OnceLock::new();
static WRITER: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

/// One thing the writer thread has to do.
pub(crate) enum Message {
    Line(String),
    /// Reach the file, answer, and end the thread.
    Shutdown(Sender<()>),
    /// Reach the file and answer, leaving the thread running: what a test needs mid-run.
    #[cfg(test)]
    Flush(Sender<()>),
}

use std::sync::mpsc::Sender;

/// The logger: a filter and a channel, and no file.
pub(crate) struct Sink {
    tx: SyncSender<Message>,
    level: LevelFilter,
    /// Records this sink could not hand over because the channel was full.
    dropped: Arc<AtomicU64>,
}

impl Sink {
    pub(crate) fn new(tx: SyncSender<Message>, level: LevelFilter) -> Self {
        Self {
            tx,
            level,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    /// How many records were lost to a full channel so far.
    #[cfg(test)]
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// The counter the writer thread reports and resets.
    pub(crate) fn counter(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.dropped)
    }

    /// The line a record becomes: time, level, target, message.
    pub(crate) fn line(record: &Record<'_>) -> String {
        format!(
            "{} {:<5} {}: {}",
            timestamp(now_ms()),
            record.level(),
            record.target(),
            record.args()
        )
    }
}

impl Log for Sink {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match self.tx.try_send(Message::Line(Self::line(record))) {
            Ok(()) => {}
            // Saturation is a cost of never blocking the frame thread: count it, and the writer
            // reports the count as soon as it can write again.
            Err(TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    }

    /// Never blocks: `log` calls this from wherever a record was made, and the writer thread owns
    /// the file. The quit path calls [`shutdown`] instead.
    fn flush(&self) {}
}

/// Install the logger for the rest of the process, rotating an oversized file first.
///
/// Returns the path it writes to. Calling it twice installs nothing the second time and answers
/// the same path, so a test that runs the app twice in one process stays honest.
pub fn install(logs: &Path, level: LevelFilter) -> std::io::Result<PathBuf> {
    let path = logs.join(FILE_NAME);
    if SENDER.get().is_some() {
        return Ok(path);
    }
    std::fs::create_dir_all(logs)?;
    rotate(&path)?;
    let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
    let sink = Sink::new(tx.clone(), level);
    let dropped = sink.counter();
    let writer = std::thread::Builder::new()
        .name("typr-log".to_string())
        .spawn(move || write_loop(rx, &path, dropped))?;
    *WRITER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(writer);
    if let Err(error) = log::set_boxed_logger(Box::new(sink)) {
        log::warn!("a logger was already installed, so typr's log file stays empty: {error}");
    }
    log::set_max_level(level);
    let _ = SENDER.set(tx);
    Ok(logs.join(FILE_NAME))
}

/// The level from `TYPR_LOG`, `warn` when it is unset or unreadable.
pub fn level_from_env() -> LevelFilter {
    level_from(std::env::var("TYPR_LOG").ok().as_deref())
}

/// The level a value names, `warn` when it names none.
pub(crate) fn level_from(value: Option<&str>) -> LevelFilter {
    value
        .and_then(|text| text.trim().parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Warn)
}

/// Every cause beneath an error, in order, on one line.
pub fn describe(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(error) = cause {
        text.push_str(": ");
        text.push_str(&error.to_string());
        cause = error.source();
    }
    text
}

/// Ask the writer to reach the file and wait, briefly, for it.
///
/// Called from `main` after `rune::run` returns, never from a destructor: the record that matters
/// most is written during teardown, so the flush has to come after it.
pub fn shutdown() {
    let Some(tx) = SENDER.get() else {
        return;
    };
    let (reply_tx, reply_rx) = mpsc::channel();
    if tx.send(Message::Shutdown(reply_tx)).is_err() {
        return;
    }
    if reply_rx.recv_timeout(FLUSH_TIMEOUT).is_err() {
        eprintln!("typr: the log writer did not reach the file before the quit");
    }
    let writer = WRITER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(writer) = writer {
        let _ = writer.join();
    }
}

/// Rename an oversized log, keeping exactly one previous copy.
fn rotate(path: &Path) -> std::io::Result<()> {
    let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if size > SIZE_LIMIT {
        let rotated = path.with_file_name(ROTATED_NAME);
        let _ = std::fs::remove_file(&rotated);
        std::fs::rename(path, rotated)?;
    }
    Ok(())
}

/// The writer: the only code that touches the file.
fn write_loop(rx: Receiver<Message>, path: &Path, dropped: Arc<AtomicU64>) {
    let mut file = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("typr: the log file could not be opened: {error}");
            return;
        }
    };
    while let Ok(message) = rx.recv() {
        match message {
            Message::Line(line) => {
                if let Err(error) = write_line(&mut file, &line, &dropped) {
                    eprintln!("typr: the log line was lost: {error}");
                }
            }
            #[cfg(test)]
            Message::Flush(reply) => {
                let _ = file.flush();
                let _ = reply.send(());
            }
            Message::Shutdown(reply) => {
                let _ = file.flush();
                let _ = reply.send(());
                return;
            }
        }
    }
    let _ = file.flush();
}

/// Write one line, reporting how many records were dropped since the last successful write.
fn write_line(file: &mut File, line: &str, counter: &AtomicU64) -> std::io::Result<()> {
    let dropped = counter.swap(0, Ordering::Relaxed);
    if dropped > 0 {
        writeln!(
            file,
            "{} WARN  typr::logging: {dropped} records dropped",
            timestamp(now_ms())
        )?;
    }
    writeln!(file, "{line}")
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// `YYYY-MM-DD HH:MM:SS.mmm` in the machine's zone, from an epoch millisecond.
fn timestamp(at_ms: i64) -> String {
    use time::OffsetDateTime;
    let offset =
        time::UtcOffset::from_whole_seconds(i32::try_from(local_offset_ms() / 1000).unwrap_or(0))
            .unwrap_or(time::UtcOffset::UTC);
    let at = OffsetDateTime::from_unix_timestamp_nanos(i128::from(at_ms) * 1_000_000)
        .map(|at| at.to_offset(offset))
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        at.year(),
        at.month() as u8,
        at.day(),
        at.hour(),
        at.minute(),
        at.second(),
        at.millisecond()
    )
}

#[cfg(test)]
mod tests;

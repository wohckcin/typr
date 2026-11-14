# Spec Delta

## Purpose

Records what typr does about its own failures: which failures reach the screen, what is written to a log file under the application data folder, and the guarantee that a stoppage in the background library work is neither silent nor a hang.

## ADDED Requirements

### Requirement: A stoppage in the background library work is reported on screen

When the background library work stops because it failed, the application SHALL report it: exactly one error message appears on the failure line, naming the stoppage, and the interface remains usable. The application MUST NOT show the same stoppage more than once per failure, and an interface that has been left with a pending answer SHALL become answerable again.

#### Scenario: A failed library worker says so

- **WHEN** the background library work panics while applying a command
- **THEN** the failure line shows a message that says the library stopped, once

#### Scenario: The interface keeps working after the stoppage

- **WHEN** the background library work has stopped and the user opens another deck
- **THEN** the application stays on screen and every later action answers with a failure message instead of waiting

### Requirement: Every accepted request receives a final answer

A request the application accepted SHALL receive exactly one answer, whether it succeeded or failed. This includes the request that was being applied when a failure occurred and every request that had been accepted but not yet applied. No request MAY be left without an answer.

#### Scenario: A grade that was being applied is answered

- **WHEN** a grade is being written when the background library work fails
- **THEN** that grade's request receives a failure answer and the card becomes answerable again

#### Scenario: Queued requests are answered after a failure

- **WHEN** three commands have been accepted and the second one is being applied when the background library work fails
- **THEN** the first had its answer, and the second and third each receive a failure answer

### Requirement: The backend reports failures without logging

The backend SHALL report a stoppage only through the answers and events it already returns, and SHALL NOT write log records, open log files or format timestamps itself, so it keeps the dependency set `project-structure` requires.

#### Scenario: A stoppage reaches the screen through an event

- **WHEN** the background library work panics
- **THEN** the backend returns one event carrying the panic message as a typed error, and writes nothing to disk itself

#### Scenario: The backend gains no logging crate

- **WHEN** the backend's normal dependency graph is listed
- **THEN** it names no logging crate, and no file or directory helper from the desktop toolkit

### Requirement: Failures are recorded in one log file

The application SHALL append failure records to a log file inside the application's logs folder. The file SHALL be bounded: when it exceeds a fixed size, the application SHALL keep the previous file as one rotated copy and start a new one, so at most two files exist. The log path SHALL follow the same root the rest of the application's data uses.

#### Scenario: A failure is written to the log

- **WHEN** the library cannot be opened at startup
- **THEN** the log file contains a record of that failure, under the logs folder of the configured root

#### Scenario: The log does not grow without limit

- **WHEN** the application starts with a log file larger than its size limit
- **THEN** the previous file becomes the rotated copy and a new log file starts empty

### Requirement: A failure record carries the cause chain

A logged failure SHALL record every cause beneath it, in order, so a reader can follow it to the underlying error. The failure line on screen SHALL keep a single line; the extra detail SHALL appear only in the log.

#### Scenario: A denied write is recorded with its cause

- **WHEN** a write fails because the operating system refused it
- **THEN** the log record names typr's error and the operating-system error beneath it, while the screen shows one line

### Requirement: The log level is chosen at startup

The application SHALL filter what it logs by a level taken from the `TYPR_LOG` environment variable, defaulting to `warn` when it is unset or unreadable. The level SHALL apply to records typr writes and MUST NOT change the filtering of libraries it depends on.

#### Scenario: The default level hides informational records

- **WHEN** the application starts with `TYPR_LOG` unset
- **THEN** the log holds failures but no startup facts

#### Scenario: The level can be raised for a diagnosis

- **WHEN** the application starts with `TYPR_LOG=info`
- **THEN** the log holds the startup facts as well as the failures

### Requirement: Startup records the facts a report needs

At information level the application SHALL record the database path, the stored schema version read before migration and the one written after it, and the number of migrations applied, so a report can distinguish a migrated library from a refused or unopened one. The backend SHALL supply those numbers as data rather than as log records.

#### Scenario: A migrated library is identifiable

- **WHEN** a library from an older schema version is opened at information level
- **THEN** the log records both the version found and the version written, and how many migrations ran

#### Scenario: A refused library is identifiable

- **WHEN** a library with a newer schema version is refused at information level
- **THEN** the log records the version found and that opening was refused

### Requirement: A slow command is recorded with its cost

A command whose answer takes longer than a fixed threshold SHALL be logged with the time it took and which command it was, so a slow interface can be traced to a command rather than guessed at. The measurement SHALL be taken where the application waits for the answer, not inside the backend. Commands below the threshold SHALL NOT be logged.

#### Scenario: A slow write is recorded

- **WHEN** a single command takes longer than the threshold
- **THEN** the log holds one record naming that command and its duration

### Requirement: A worker that ended abnormally is recorded when the application quits

When the application quits, a background library worker that ended by failing SHALL be recorded in the log, so the record survives even when the screen that would have shown it is already gone. The record SHALL be written after the worker's end is known and before the log is flushed.

#### Scenario: Quitting after a failure leaves a record

- **WHEN** the library worker failed earlier and the user quits the application
- **THEN** the log holds a record of the abnormal end

### Requirement: Logging never stalls the interface

Recording a log entry SHALL NOT perform file work on the thread that draws the interface, and SHALL NOT block that thread when the log cannot keep up. When entries are dropped because the log is saturated, the count SHALL be reported in a later log entry rather than lost silently.

#### Scenario: A burst of records does not stall drawing

- **WHEN** many records are written from the interface thread at once
- **THEN** the interface keeps drawing, and the entry count eventually reported accounts for any dropped records

#### Scenario: Logging at quit still lands

- **WHEN** the application quits immediately after recording a failure
- **THEN** the record is present in the log file after the process exits

// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! WHATWG Streams API — ReadableStream, WritableStream, TransformStream.
//!
//! Provides in-memory stream primitives for the Fetch body consumption,
//! progressive rendering, and general streaming data processing.
//! <https://streams.spec.whatwg.org/>

use std::collections::VecDeque;

// ── ReadableStream ───────────────────────────────────────────────────────────

/// State of a readable stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadableStreamState {
    /// Stream is open and readable.
    #[default]
    Readable,
    /// Stream has been fully consumed.
    Closed,
    /// Stream has been cancelled or errored.
    Errored,
}

/// A chunk of data in a stream.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A chunk of bytes.
    Bytes(Vec<u8>),
    /// A text string chunk.
    Text(String),
}

impl StreamChunk {
    pub fn byte_length(&self) -> usize {
        match self {
            Self::Bytes(b) => b.len(),
            Self::Text(s) => s.len(),
        }
    }
}

/// A ReadableStream that holds chunks in memory.
#[derive(Debug)]
pub struct ReadableStream {
    state: ReadableStreamState,
    queue: VecDeque<StreamChunk>,
    total_bytes_enqueued: usize,
    total_bytes_read: usize,
    high_water_mark: usize,
    locked: bool,
}

impl Default for ReadableStream {
    fn default() -> Self {
        Self::new(65536) // 64KB default high water mark
    }
}

impl ReadableStream {
    /// Create a new readable stream with a given high water mark.
    pub fn new(high_water_mark: usize) -> Self {
        Self {
            state: ReadableStreamState::Readable,
            queue: VecDeque::new(),
            total_bytes_enqueued: 0,
            total_bytes_read: 0,
            high_water_mark,
            locked: false,
        }
    }

    pub fn state(&self) -> ReadableStreamState {
        self.state
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Lock the stream (e.g., when a reader is acquired).
    pub fn lock(&mut self) -> bool {
        if self.locked {
            return false;
        }
        self.locked = true;
        true
    }

    /// Unlock the stream.
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Enqueue a chunk into the stream.
    pub fn enqueue(&mut self, chunk: StreamChunk) -> Result<(), &'static str> {
        if self.state != ReadableStreamState::Readable {
            return Err("stream is not readable");
        }
        self.total_bytes_enqueued += chunk.byte_length();
        self.queue.push_back(chunk);
        Ok(())
    }

    /// Read one chunk from the stream.
    pub fn read(&mut self) -> Option<StreamChunk> {
        if self.state == ReadableStreamState::Errored {
            return None;
        }
        let chunk = self.queue.pop_front()?;
        self.total_bytes_read += chunk.byte_length();
        Some(chunk)
    }

    /// Close the stream (no more chunks will be enqueued).
    pub fn close(&mut self) {
        if self.state == ReadableStreamState::Readable {
            self.state = ReadableStreamState::Closed;
        }
    }

    /// Cancel the stream with a reason.
    pub fn cancel(&mut self) {
        self.state = ReadableStreamState::Errored;
        self.queue.clear();
    }

    /// Whether there are chunks available.
    pub fn has_pending_chunks(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Number of chunks currently in the queue.
    pub fn queued_chunks(&self) -> usize {
        self.queue.len()
    }

    /// Total bytes in the queue.
    pub fn queued_size(&self) -> usize {
        self.queue.iter().map(|c| c.byte_length()).sum()
    }

    /// Whether backpressure should be applied (queue size >= high water mark).
    pub fn should_apply_backpressure(&self) -> bool {
        self.queued_size() >= self.high_water_mark
    }

    /// Get total bytes enqueued over the stream's lifetime.
    pub fn total_bytes_enqueued(&self) -> usize {
        self.total_bytes_enqueued
    }

    /// Get total bytes read.
    pub fn total_bytes_read(&self) -> usize {
        self.total_bytes_read
    }

    /// Read all available chunks into a single byte vec.
    pub fn read_all_bytes(&mut self) -> Vec<u8> {
        let mut result = Vec::new();
        while let Some(chunk) = self.read() {
            match chunk {
                StreamChunk::Bytes(b) => result.extend_from_slice(&b),
                StreamChunk::Text(s) => result.extend_from_slice(s.as_bytes()),
            }
        }
        result
    }

    /// Pipe contents to a writable stream.
    pub fn pipe_to(&mut self, destination: &mut WritableStream) -> Result<usize, &'static str> {
        if self.state == ReadableStreamState::Errored {
            return Err("source stream errored");
        }
        if destination.state() == WritableStreamState::Errored {
            return Err("destination stream errored");
        }
        let mut transferred = 0;
        while let Some(chunk) = self.read() {
            transferred += chunk.byte_length();
            destination.write(chunk)?;
        }
        Ok(transferred)
    }

    /// Tee (split) this stream into two new streams.
    pub fn tee(&mut self) -> Result<(ReadableStream, ReadableStream), &'static str> {
        if self.locked {
            return Err("stream is locked");
        }
        let mut branch1 = ReadableStream::new(self.high_water_mark);
        let mut branch2 = ReadableStream::new(self.high_water_mark);

        while let Some(chunk) = self.read() {
            let chunk_clone = chunk.clone();
            branch1.enqueue(chunk)?;
            branch2.enqueue(chunk_clone)?;
        }

        if self.state == ReadableStreamState::Closed {
            branch1.close();
            branch2.close();
        }

        Ok((branch1, branch2))
    }
}

// ── WritableStream ───────────────────────────────────────────────────────────

/// State of a writable stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritableStreamState {
    /// Stream is open and writable.
    #[default]
    Writable,
    /// Stream has been closed.
    Closed,
    /// Stream has errored.
    Errored,
}

/// A WritableStream that collects chunks.
#[derive(Debug)]
pub struct WritableStream {
    state: WritableStreamState,
    chunks: Vec<StreamChunk>,
    total_bytes_written: usize,
    locked: bool,
}

impl Default for WritableStream {
    fn default() -> Self {
        Self::new()
    }
}

impl WritableStream {
    pub fn new() -> Self {
        Self {
            state: WritableStreamState::Writable,
            chunks: Vec::new(),
            total_bytes_written: 0,
            locked: false,
        }
    }

    pub fn state(&self) -> WritableStreamState {
        self.state
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Write a chunk to the stream.
    pub fn write(&mut self, chunk: StreamChunk) -> Result<(), &'static str> {
        if self.state != WritableStreamState::Writable {
            return Err("stream is not writable");
        }
        self.total_bytes_written += chunk.byte_length();
        self.chunks.push(chunk);
        Ok(())
    }

    /// Close the stream.
    pub fn close(&mut self) {
        if self.state == WritableStreamState::Writable {
            self.state = WritableStreamState::Closed;
        }
    }

    /// Abort the stream.
    pub fn abort(&mut self) {
        self.state = WritableStreamState::Errored;
    }

    pub fn total_bytes_written(&self) -> usize {
        self.total_bytes_written
    }

    /// Get all written chunks.
    pub fn chunks(&self) -> &[StreamChunk] {
        &self.chunks
    }

    /// Consume all chunks into a single byte vec.
    pub fn into_bytes(self) -> Vec<u8> {
        let mut result = Vec::new();
        for chunk in self.chunks {
            match chunk {
                StreamChunk::Bytes(b) => result.extend_from_slice(&b),
                StreamChunk::Text(s) => result.extend_from_slice(s.as_bytes()),
            }
        }
        result
    }
}

// ── TransformStream ──────────────────────────────────────────────────────────

/// A function that transforms chunks.
type TransformFn = Box<dyn Fn(StreamChunk) -> StreamChunk + Send>;

/// TransformStream — takes a readable side, transforms chunks, and writes to writable side.
pub struct TransformStream {
    readable: ReadableStream,
    writable: WritableStream,
    transform: Option<TransformFn>,
}

impl TransformStream {
    /// Create a transform stream with identity transform.
    pub fn new() -> Self {
        Self {
            readable: ReadableStream::default(),
            writable: WritableStream::new(),
            transform: None,
        }
    }

    /// Create with a custom transform function.
    pub fn with_transform<F>(transform: F) -> Self
    where
        F: Fn(StreamChunk) -> StreamChunk + Send + 'static,
    {
        Self {
            readable: ReadableStream::default(),
            writable: WritableStream::new(),
            transform: Some(Box::new(transform)),
        }
    }

    /// Write a chunk to the writable side; the transformed result is enqueued on the readable side.
    pub fn write(&mut self, chunk: StreamChunk) -> Result<(), &'static str> {
        let transformed = match &self.transform {
            Some(f) => f(chunk),
            None => chunk,
        };
        self.readable.enqueue(transformed)
    }

    /// Read from the readable side.
    pub fn read(&mut self) -> Option<StreamChunk> {
        self.readable.read()
    }

    /// Close both sides.
    pub fn close(&mut self) {
        self.writable.close();
        self.readable.close();
    }

    pub fn readable(&self) -> &ReadableStream {
        &self.readable
    }

    pub fn writable(&self) -> &WritableStream {
        &self.writable
    }
}

impl Default for TransformStream {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_stream_basic() {
        let mut stream = ReadableStream::default();
        assert_eq!(stream.state(), ReadableStreamState::Readable);
        assert!(!stream.is_locked());
        assert_eq!(stream.queued_chunks(), 0);

        stream.enqueue(StreamChunk::Bytes(vec![1, 2, 3])).unwrap();
        stream.enqueue(StreamChunk::Text("hello".into())).unwrap();
        assert_eq!(stream.queued_chunks(), 2);
        assert_eq!(stream.queued_size(), 8); // 3 + 5
    }

    #[test]
    fn readable_stream_read() {
        let mut stream = ReadableStream::default();
        stream.enqueue(StreamChunk::Bytes(vec![42])).unwrap();
        stream.enqueue(StreamChunk::Text("world".into())).unwrap();

        let chunk = stream.read().unwrap();
        assert!(matches!(chunk, StreamChunk::Bytes(ref b) if b == &[42]));
        assert_eq!(stream.total_bytes_read(), 1);

        let chunk = stream.read().unwrap();
        assert!(matches!(chunk, StreamChunk::Text(ref s) if s == "world"));
        assert_eq!(stream.total_bytes_read(), 6);

        assert!(stream.read().is_none());
    }

    #[test]
    fn readable_stream_close() {
        let mut stream = ReadableStream::default();
        stream.enqueue(StreamChunk::Bytes(vec![1])).unwrap();
        stream.close();
        assert_eq!(stream.state(), ReadableStreamState::Closed);
        // Can still read queued data after close
        assert!(stream.read().is_some());
        // But can't enqueue new data
        assert!(stream.enqueue(StreamChunk::Bytes(vec![2])).is_err());
    }

    #[test]
    fn readable_stream_cancel() {
        let mut stream = ReadableStream::default();
        stream.enqueue(StreamChunk::Bytes(vec![1, 2])).unwrap();
        stream.cancel();
        assert_eq!(stream.state(), ReadableStreamState::Errored);
        assert!(!stream.has_pending_chunks());
    }

    #[test]
    fn readable_stream_lock() {
        let mut stream = ReadableStream::default();
        assert!(stream.lock());
        assert!(stream.is_locked());
        assert!(!stream.lock()); // Double lock fails
        stream.unlock();
        assert!(!stream.is_locked());
    }

    #[test]
    fn readable_stream_backpressure() {
        let mut stream = ReadableStream::new(10);
        assert!(!stream.should_apply_backpressure());
        stream.enqueue(StreamChunk::Bytes(vec![0; 10])).unwrap();
        assert!(stream.should_apply_backpressure());
    }

    #[test]
    fn readable_stream_read_all_bytes() {
        let mut stream = ReadableStream::default();
        stream.enqueue(StreamChunk::Bytes(vec![1, 2])).unwrap();
        stream.enqueue(StreamChunk::Text("AB".into())).unwrap();
        let all = stream.read_all_bytes();
        assert_eq!(all, vec![1, 2, 65, 66]); // 65='A', 66='B'
    }

    #[test]
    fn readable_stream_tee() {
        let mut stream = ReadableStream::default();
        stream.enqueue(StreamChunk::Bytes(vec![1, 2])).unwrap();
        stream.enqueue(StreamChunk::Bytes(vec![3, 4])).unwrap();
        stream.close();

        let (mut b1, mut b2) = stream.tee().unwrap();
        assert_eq!(b1.state(), ReadableStreamState::Closed);
        assert_eq!(b2.state(), ReadableStreamState::Closed);

        let d1 = b1.read_all_bytes();
        let d2 = b2.read_all_bytes();
        assert_eq!(d1, vec![1, 2, 3, 4]);
        assert_eq!(d2, vec![1, 2, 3, 4]);
    }

    #[test]
    fn writable_stream_basic() {
        let mut stream = WritableStream::new();
        assert_eq!(stream.state(), WritableStreamState::Writable);
        stream.write(StreamChunk::Bytes(vec![1, 2, 3])).unwrap();
        stream.write(StreamChunk::Text("hello".into())).unwrap();
        assert_eq!(stream.total_bytes_written(), 8);
        assert_eq!(stream.chunks().len(), 2);
    }

    #[test]
    fn writable_stream_close() {
        let mut stream = WritableStream::new();
        stream.write(StreamChunk::Bytes(vec![1])).unwrap();
        stream.close();
        assert!(stream.write(StreamChunk::Bytes(vec![2])).is_err());
    }

    #[test]
    fn writable_stream_abort() {
        let mut stream = WritableStream::new();
        stream.abort();
        assert_eq!(stream.state(), WritableStreamState::Errored);
        assert!(stream.write(StreamChunk::Bytes(vec![1])).is_err());
    }

    #[test]
    fn writable_stream_into_bytes() {
        let mut stream = WritableStream::new();
        stream.write(StreamChunk::Bytes(vec![1, 2])).unwrap();
        stream.write(StreamChunk::Text("CD".into())).unwrap();
        let all = stream.into_bytes();
        assert_eq!(all, vec![1, 2, 67, 68]); // 67='C', 68='D'
    }

    #[test]
    fn pipe_to() {
        let mut src = ReadableStream::default();
        src.enqueue(StreamChunk::Bytes(vec![1, 2, 3])).unwrap();
        src.enqueue(StreamChunk::Text("hi".into())).unwrap();

        let mut dst = WritableStream::new();
        let transferred = src.pipe_to(&mut dst).unwrap();
        assert_eq!(transferred, 5);
        assert_eq!(dst.total_bytes_written(), 5);
    }

    #[test]
    fn transform_stream_identity() {
        let mut ts = TransformStream::new();
        ts.write(StreamChunk::Bytes(vec![1, 2, 3])).unwrap();
        let chunk = ts.read().unwrap();
        assert!(matches!(chunk, StreamChunk::Bytes(ref b) if b == &[1, 2, 3]));
    }

    #[test]
    fn transform_stream_with_transform() {
        let mut ts = TransformStream::with_transform(|chunk| match chunk {
            StreamChunk::Text(s) => StreamChunk::Text(s.to_uppercase()),
            other => other,
        });
        ts.write(StreamChunk::Text("hello".into())).unwrap();
        let chunk = ts.read().unwrap();
        assert!(matches!(chunk, StreamChunk::Text(ref s) if s == "HELLO"));
    }

    #[test]
    fn transform_stream_close() {
        let mut ts = TransformStream::new();
        ts.write(StreamChunk::Bytes(vec![1])).unwrap();
        ts.close();
        assert_eq!(ts.readable().state(), ReadableStreamState::Closed);
        assert_eq!(ts.writable().state(), WritableStreamState::Closed);
    }

    #[test]
    fn chunk_byte_length() {
        assert_eq!(StreamChunk::Bytes(vec![1, 2, 3]).byte_length(), 3);
        assert_eq!(StreamChunk::Text("hello".into()).byte_length(), 5);
    }
}

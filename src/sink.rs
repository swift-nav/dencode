/// An `IterSink` is a value into which other values can be sent.
///
/// Values are sent in two phases: first by internally buffering the value,
/// and then actually writing the value upon flushing.
pub trait IterSink<Item> {
    /// The type of value produced by the sink when an error occurs.
    type Error;

    /// Attempts to prepare the `IterSink` to receive a value, adjusting the internal
    /// buffer if necessary.
    ///
    /// This method must be called prior to each call to `start_send`.
    fn ready(&mut self) -> Result<(), Self::Error>;

    /// Write a value to the internal buffer.
    /// Each call to this function must be preceded by a successful call to `ready`.
    fn start_send(&mut self, item: Item) -> Result<(), Self::Error>;

    /// Flush any remaining output from this sink's internal buffer.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// An extension trait for `IterSink`s that provides a few convenient functions.
pub trait IterSinkExt<Item>: IterSink<Item> {
    /// Fully processed an item into the sink, including flushing.
    ///
    /// Note that, because of the flushing requirement, it is usually better to batch
    /// together items to send via send_all, rather than flushing between each item.
    fn send(&mut self, item: Item) -> Result<(), Self::Error> {
        self.send_all(std::iter::once(Ok(item)))
    }

    /// Fully process the iterator of items into the sink, including flushing.
    ///
    /// This will drive the iterator to keep producing items until it is exhausted, sending each item to the sink.
    /// It will complete once both the input iterator is exhausted, and the sink has
    /// received and flushed all items.
    fn send_all<I>(&mut self, items: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Result<Item, Self::Error>>,
    {
        for item in items {
            let item = item?;
            self.ready()?;
            self.start_send(item)?;
        }
        self.flush()
    }
}

impl<T: ?Sized, Item> IterSinkExt<Item> for T where T: IterSink<Item> {}

use std::borrow::Borrow;
use std::cell::RefCell;
use std::iter::Iterator;
use std::rc::Rc;

#[cfg(any(test, feature = "slice-deque"))]
use slice_deque::SliceDeque;

/// A stream of context/value pairs that can be subscribed to.
///
/// Note: in order to use stream trait methods, this trait
/// must imported into the current scope.
pub trait Stream<'a>: Sized {
    /// The type of the context attached to emitted elements.
    ///
    /// Can be set to `()` to ignore the context part of the stream.
    type Context: ?Sized;

    /// The type of the elements being emitted.
    type Item: ?Sized;

    /// Attaches an observer (a user-provided mutable closure) to the
    /// stream, which consumes the stream object.
    ///
    /// A stream with no observer attached is essentially just
    /// a function of an observer; it will not react to incoming events
    /// until it is subscribed to.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*; use std::cell::RefCell;
    /// let out = RefCell::new(Vec::new());
    /// let stream = SimpleBroadcast::<i64>::new();
    /// stream
    ///     .clone()
    ///     .filter(|x| x % 2 != 0)
    ///     .subscribe(|x| out.borrow_mut().push(*x));
    /// stream.feed(0..=5);
    /// assert_eq!(&*out.borrow(), &[1, 3, 5]);
    /// ```
    fn subscribe<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Item),
    {
        self.subscribe_ctx(move |_ctx, item| observer(item))
    }

    /// Same as `subscribe()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*; use std::cell::Cell;
    /// let result = Cell::new((0, 0.));
    /// let stream = Broadcast::<i32, f64>::new();
    /// stream
    ///     .clone()
    ///     .map_ctx(|c, x| (*c as f64) + *x)
    ///     .subscribe_ctx(|c, x| result.set((*c, *x)));
    /// stream.send_ctx(3, 7.5);
    /// assert_eq!(result.get(), (3, 10.5));
    /// ```
    fn subscribe_ctx<O>(self, observer: O)
    where
        O: 'a + FnMut(&Self::Context, &Self::Item);

    /// Create a broadcast from a stream, enabling multiple observers. This is the only
    /// `Stream` trait method that incurs a slight runtime cost, due to the broadcast
    /// object having to store observers as boxed trait objects in a reference-counted
    /// container; all other methods can be inlined.
    ///
    /// Note: this is equivalent to creating a broadcast via
    /// [`from_stream()`](struct.Broadcast.html#provided-methods)
    /// constructor.
    fn broadcast(self) -> Broadcast<'a, Self::Context, Self::Item>
    where
        Self: 'a,
    {
        Broadcast::from_stream(self)
    }

    /// Convenience method to extract the context into a separate stream.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<i32, String>::new();
    /// let double_ctx = stream.ctx().map(|x| x * 2);
    /// ```
    ///
    /// # Notes
    ///
    /// - Resulting stream's context/value will reference the same object
    ///   (original stream's context).
    /// - The return value is a `Stream` object (both context/value types
    ///   are the original stream's context type).
    fn ctx(self) -> Context<Self> {
        Context { stream: self }
    }

    /// Set the context to a fixed constant value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<String>::new();
    /// let stream_with_ctx = stream.with_ctx(42);
    /// ```
    ///
    /// # Notes
    ///
    /// - The value is passed down unchanged.
    /// - The return value is a `Stream` object (context type is the type
    ///   of the provided value; value type is unchanged).
    fn with_ctx<T>(self, ctx: T) -> WithContext<Self, T> {
        WithContext { stream: self, ctx }
    }

    /// Creates a new stream which calls a closure on each context/value and uses
    /// that as the context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<String>::new();
    /// let string_and_len = stream.with_ctx_map(|_, s| s.len());
    /// ```
    ///
    /// # Notes
    ///
    /// - The value is passed down unchanged.
    /// - The closure receives all of its arguments by reference.
    /// - The return value is a `Stream` object (context type is the return
    ///   type of the closure; value type is unchanged).
    fn with_ctx_map<F, T>(self, func: F) -> WithContextMap<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> T,
    {
        WithContextMap { stream: self, func }
    }

    /// Creates a new stream which calls a closure on each element and uses
    /// that as the value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<String>::new();
    /// let contains_foo = stream.map(|s| s.contains("foo"));
    /// ```
    ///
    /// # Notes
    ///
    /// - The context is passed down unchanged.
    /// - The closure receives its argument by reference.
    /// - The return value is a `Stream` object (context type is unchanged;
    ///   value type is the return type of the closure).
    fn map<F, T>(self, func: F) -> Map<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> T,
    {
        Map { stream: self, func: NoContext(func) }
    }

    /// Same as `map()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<bool, i32>::new();
    /// let div2 = stream.map_ctx(|c, x| (*x % 2 == 0) == *c);
    /// ```
    fn map_ctx<F, T>(self, func: F) -> Map<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> T,
    {
        Map { stream: self, func }
    }

    /// Same as `map()`, but the closure is expected to return a `(context, value)`
    /// tuple, so that both the context and the value can be changed at the same time.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*; type A = i32; type B = u32;
    /// let stream = SimpleBroadcast::<i32>::new();
    /// let string_context = stream.map_both(|x| (x.to_string(), *x));
    /// ```
    ///
    /// # Notes
    ///
    /// - The context and the value are changed simultaneously.
    /// - The closure receives its argument by reference.
    /// - The return value is a `Stream` object (context type and value type
    ///   depend on the return type of the closure).
    fn map_both<F, C, T>(self, func: F) -> MapBoth<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> (C, T),
    {
        MapBoth { stream: self, func: NoContext(func) }
    }

    /// Same as `map_both()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*; type A = i32; type B = u32;
    /// let stream = Broadcast::<A, B>::new();
    /// let swapped = stream.map_both_ctx(|a, b| (*a, *b));
    /// ```
    fn map_both_ctx<F, C, T>(self, func: F) -> MapBoth<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> (C, T),
    {
        MapBoth { stream: self, func }
    }

    /// Creates a stream which uses a closure to determine if an element should be
    /// yielded.
    ///
    /// The closure must return `true` or `false` and is called on each element of the
    /// original stream. If `true` is returned, the element is passed downstream.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<Vec<i32>>::new();
    /// let non_empty = stream.filter(|v| !v.is_empty());
    /// ```
    ///
    /// # Notes
    ///
    /// - The context is passed down unchanged.
    /// - The closure receives its argument by reference.
    /// - The return value is a `Stream` object (same context type and
    ///   value type as the original stream).
    fn filter<F>(self, func: F) -> Filter<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> bool,
    {
        Filter { stream: self, func: NoContext(func) }
    }

    /// Same as `filter()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<usize, Vec<i32>>::new();
    /// let filter_len = stream.filter_ctx(|ctx, v| v.len() == *ctx);
    /// ```
    fn filter_ctx<F>(self, func: F) -> Filter<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> bool,
    {
        Filter { stream: self, func }
    }

    /// Creates a stream that both filters and maps.
    ///
    /// The closure must return an `Option<T>`. If it returns `Some(element)`, then
    /// that element is returned; otherwise it is skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<String>::new();
    /// let valid_ints = stream.filter_map(|s| s.parse::<i64>().ok());
    /// ```
    ///
    /// # Notes
    ///
    /// - The context is passed down unchanged.
    /// - The closure receives its argument by reference.
    /// - The return value is a `Stream` object (context type is unchanged;
    ///   value type is the is `T` if the return type of the closure is `Option<T>`).
    fn filter_map<F, T>(self, func: F) -> FilterMap<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> Option<T>,
    {
        FilterMap { stream: self, func: NoContext(func) }
    }

    /// Same as `filter_map()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<Option<i64>, String>::new();
    /// let int_or_ctx = stream.filter_map_ctx(|c, s| s.parse().ok().or(*c));
    /// ```
    fn filter_map_ctx<F, T>(self, func: F) -> FilterMap<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> Option<T>,
    {
        FilterMap { stream: self, func }
    }

    /// 'Reduce' operation on streams.
    ///
    /// This method takes two arguments: an initial value, and a closure with
    /// two arguments: an accumulator and an element. The closure returns the
    /// value that the accumulator should have for the next iteration; the
    /// initial value is the value the accumulator will have on the first call.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<i32>::new();
    /// let cum_sum = stream.fold(0, |acc, x| acc + x);
    /// ```
    ///
    /// # Notes
    ///
    /// - The context is passed down unchanged.
    /// - The closure receives all of its arguments by reference.
    /// - The return value is a `Stream` object (context type is unchanged;
    ///   value type is the accumulator type).
    fn fold<F, T: 'a>(self, init: T, func: F) -> Fold<Self, NoContext<F>, T>
    where
        F: 'a + FnMut(&T, &Self::Item) -> T,
    {
        Fold { stream: self, init, func: NoContext(func) }
    }

    /// Same as `fold()`, but the closure receives three arguments
    /// (context/accumulator/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<i32, i32>::new();
    /// let bnd_sum = stream.fold_ctx(0, |c, acc, x| *c.min(&(acc + x)));
    /// ```
    fn fold_ctx<F, T: 'a>(self, init: T, func: F) -> Fold<Self, F, T>
    where
        F: 'a + FnMut(&Self::Context, &T, &Self::Item) -> T,
    {
        Fold { stream: self, init, func }
    }

    /// Do something with each element of a stream, passing the value on.
    ///
    /// The closure will only be called if the stream is actually
    /// subscribed to (just calling `inspect()` does nothing on its own).
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<String>::new();
    /// let stream = stream.inspect(|x| println!("{:?}", x));
    /// ```
    ///
    /// # Notes
    ///
    /// - Both context/value are passed down unchanged.
    /// - The closure receives its argument by reference.
    /// - The return value is a `Stream` object (same context type and
    ///   value type as the original stream).
    fn inspect<F>(self, func: F) -> Inspect<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item),
    {
        Inspect { stream: self, func: NoContext(func) }
    }

    /// Same as `inspect()`, but the closure receives two arguments
    /// (context/value), by reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = Broadcast::<i32, String>::new();
    /// let stream = stream.inspect_ctx(|c, x| println!("{} {}", c, x));
    /// ```
    fn inspect_ctx<F>(self, func: F) -> Inspect<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item),
    {
        Inspect { stream: self, func }
    }

    /// Creates a stream that caches up to `n` last elements.
    ///
    /// The elements are stored in a contiguous double-ended
    /// queue provided via [`slice-deque`](https://crates.io/crates/slice-deque)
    /// crate. The output stream yields slice views into this queue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reactive_rs::*;
    /// let stream = SimpleBroadcast::<i64>::new();
    /// let last_3 = stream.last_n(3);
    /// ```
    ///
    /// # Notes
    ///
    /// - The context is passed down unchanged (only values are cached).
    /// - The return value is a `Stream` object (context type is unchanged;
    ///   value type is `[T]` where `T` is the original value type).
    /// - Slices may contain less than `n` elements (while the queue
    ///   is being filled up initially).
    /// - The value type of the original stream must implement `Clone`.
    /// - This method is only present if `feature = "slice-deque"` is
    ///   enabled (on by default).
    #[cfg(any(test, feature = "slice-deque"))]
    fn last_n(self, count: usize) -> LastN<Self, Self::Item>
    where
        Self::Item: 'a + Clone + Sized,
    {
        LastN { count, stream: self, data: Rc::new(RefCell::new(SliceDeque::with_capacity(count))) }
    }
}

pub trait ContextFn<C: ?Sized, T: ?Sized> {
    type Output;

    fn call_mut(&mut self, ctx: &C, item: &T) -> Self::Output;
}

impl<C: ?Sized, T: ?Sized, V, F> ContextFn<C, T> for F
where
    F: FnMut(&C, &T) -> V,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, ctx: &C, item: &T) -> Self::Output {
        self(ctx, item)
    }
}

pub trait ContextFoldFn<C: ?Sized, T: ?Sized, V> {
    type Output;

    fn call_mut(&mut self, ctx: &C, acc: &V, item: &T) -> Self::Output;
}

impl<C: ?Sized, T: ?Sized, V, F> ContextFoldFn<C, T, V> for F
where
    F: FnMut(&C, &V, &T) -> V,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, ctx: &C, acc: &V, item: &T) -> Self::Output {
        self(ctx, acc, item)
    }
}

pub struct NoContext<F>(F);

impl<F, C: ?Sized, T: ?Sized, V> ContextFn<C, T> for NoContext<F>
where
    F: FnMut(&T) -> V,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, _ctx: &C, item: &T) -> Self::Output {
        (self.0)(item)
    }
}

impl<F, C: ?Sized, T: ?Sized, V> ContextFoldFn<C, T, V> for NoContext<F>
where
    F: FnMut(&V, &T) -> V,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, _ctx: &C, acc: &Self::Output, item: &T) -> Self::Output {
        (self.0)(acc, item)
    }
}

type Callback<'a, C, T> = Box<'a + FnMut(&C, &T)>;

/// Event source that transmits context/value pairs to multiple observers.
///
/// In order to "fork" the broadcast (creating a new stream that will
/// be subscribed to it), the broadcast object can be simply cloned
/// via the `Clone` trait. Note that cloning the broadcast only
/// increases its reference count; no values are being cloned or copied.
///
/// A broadcast may receive a value in one of two ways. First, the user
/// may explicitly call one of its methods: `send()`,  `send_ctx()`,
/// `feed()`, `feed_ctx()`. Second, the broadcast may be created
/// from a parent stream via [`broadcast()`](trait.Stream.html#provided-methods)
/// method of the stream object. Either way, each context/value pair received
/// is passed on to each of the subscribed observers, by reference.
///
/// # Examples
///
/// ```
/// # use reactive_rs::*; use std::cell::RefCell; use std::rc::Rc;
/// let out = RefCell::new(Vec::new());
/// let stream = SimpleBroadcast::<i32>::new();
/// let child1 = stream
///     .clone()
///     .subscribe(|x| out.borrow_mut().push(*x + 1));
/// let child2 = stream
///     .clone()
///     .subscribe(|x| out.borrow_mut().push(*x + 7));
/// stream.feed(1..=3);
/// assert_eq!(&*out.borrow(), &[2, 8, 3, 9, 4, 10]);
/// ```
pub struct Broadcast<'a, C: ?Sized, T: ?Sized> {
    observers: Rc<RefCell<Vec<Callback<'a, C, T>>>>,
}

impl<'a, C: 'a + ?Sized, T: 'a + ?Sized> Broadcast<'a, C, T> {
    /// Creates a new broadcast with specified context and item types.
    pub fn new() -> Self {
        Self { observers: Rc::new(RefCell::new(Vec::new())) }
    }

    /// Create a broadcast from a stream, enabling multiple observers
    /// ("fork" the stream).
    ///
    /// Note: this is equivalent to calling
    /// [`broadcast()`](trait.Stream.html#provided-methods) on the stream object.
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<'a, Context = C, Item = T>,
    {
        let broadcast = Self::new();
        let clone = broadcast.clone();
        stream.subscribe_ctx(move |ctx, x| clone.send_ctx(ctx, x));
        broadcast
    }

    fn push<F>(&self, func: F)
    where
        F: FnMut(&C, &T) + 'a,
    {
        self.observers.borrow_mut().push(Box::new(func));
    }

    /// Send a value along with context to all observers of the broadcast.
    pub fn send_ctx<K, B>(&self, ctx: K, value: B)
    where
        K: Borrow<C>,
        B: Borrow<T>,
    {
        let ctx = ctx.borrow();
        let value = value.borrow();
        for observer in self.observers.borrow_mut().iter_mut() {
            observer(ctx, value);
        }
    }

    /// Similar to `send_ctx()`, but the context is set to the type's default value.
    pub fn send<B>(&self, value: B)
    where
        B: Borrow<T>,
        C: Default,
    {
        let ctx = C::default();
        self.send_ctx(&ctx, value);
    }

    /// Convenience method to feed an iterator of values to all observers of the
    /// broadcast, along with a given context.
    pub fn feed_ctx<K, B, I>(&self, ctx: K, iter: I)
    where
        K: Borrow<C>,
        I: Iterator<Item = B>,
        B: Borrow<T>,
    {
        let ctx = ctx.borrow();
        for value in iter {
            self.send_ctx(ctx, value);
        }
    }

    /// Similar to `feed_ctx()`, but the context is set to the type's default value.
    pub fn feed<B, I>(&self, iter: I)
    where
        I: Iterator<Item = B>,
        B: Borrow<T>,
        C: Default,
    {
        let ctx = C::default();
        self.feed_ctx(&ctx, iter);
    }
}

/// Simplified broadcast that only transmits values without context.
pub type SimpleBroadcast<'a, T> = Broadcast<'a, (), T>;

impl<'a, C: 'a + ?Sized, T: 'a + ?Sized> Default for Broadcast<'a, C, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, C: 'a + ?Sized, T: 'a + ?Sized> Clone for Broadcast<'a, C, T> {
    fn clone(&self) -> Self {
        Self { observers: self.observers.clone() }
    }
}

impl<'a, C: 'a + ?Sized, T: 'a + ?Sized> Stream<'a> for Broadcast<'a, C, T> {
    type Context = C;
    type Item = T;

    fn subscribe_ctx<O>(self, observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        self.push(observer);
    }
}

pub struct WithContext<S, T> {
    stream: S,
    ctx: T,
}

impl<'a, S, T: 'a> Stream<'a> for WithContext<S, T>
where
    S: Stream<'a>,
{
    type Context = T;
    type Item = S::Item;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let ctx = self.ctx;
        self.stream.subscribe_ctx(move |_ctx, x| {
            observer(&ctx, x);
        })
    }
}

pub struct WithContextMap<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F, T> Stream<'a> for WithContextMap<S, F>
where
    S: Stream<'a>,
    F: 'a + FnMut(&S::Context, &S::Item) -> T,
{
    type Context = T;
    type Item = S::Item;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            observer(&func(ctx, x), x);
        })
    }
}

pub struct Context<S> {
    stream: S,
}

impl<'a, S> Stream<'a> for Context<S>
where
    S: Stream<'a>,
{
    type Context = S::Context;
    type Item = S::Context;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        self.stream.subscribe_ctx(move |ctx, _x| {
            observer(ctx, ctx);
        })
    }
}

pub struct Map<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F> Stream<'a> for Map<S, F>
where
    S: Stream<'a>,
    F: 'a + ContextFn<S::Context, S::Item>,
{
    type Context = S::Context;
    type Item = F::Output;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| observer(ctx, &func.call_mut(ctx, x)))
    }
}

pub struct MapBoth<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F, C, T> Stream<'a> for MapBoth<S, F>
where
    S: Stream<'a>,
    F: 'a + ContextFn<S::Context, S::Item, Output = (C, T)>,
{
    type Context = C;
    type Item = T;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            let (ctx, x) = func.call_mut(ctx, x);
            observer(&ctx, &x);
        })
    }
}

pub struct Filter<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F> Stream<'a> for Filter<S, F>
where
    S: Stream<'a>,
    F: 'a + ContextFn<S::Context, S::Item, Output = bool>,
{
    type Context = S::Context;
    type Item = S::Item;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Context, &Self::Item),
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            if func.call_mut(ctx, x) {
                observer(ctx, x);
            }
        });
    }
}

pub struct FilterMap<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F, T> Stream<'a> for FilterMap<S, F>
where
    S: Stream<'a>,
    F: 'a + ContextFn<S::Context, S::Item, Output = Option<T>>,
{
    type Context = S::Context;
    type Item = T;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Context, &Self::Item),
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            if let Some(x) = func.call_mut(ctx, x) {
                observer(ctx, &x);
            }
        });
    }
}

pub struct Fold<S, F, T> {
    stream: S,
    init: T,
    func: F,
}

impl<'a, S, F, T: 'a> Stream<'a> for Fold<S, F, T>
where
    S: Stream<'a>,
    F: 'a + ContextFoldFn<S::Context, S::Item, T, Output = T>,
{
    type Context = S::Context;
    type Item = T;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut value = self.init;
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            value = func.call_mut(ctx, &value, x);
            observer(ctx, &value);
        })
    }
}

pub struct Inspect<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F> Stream<'a> for Inspect<S, F>
where
    S: Stream<'a>,
    F: 'a + ContextFn<S::Context, S::Item, Output = ()>,
{
    type Context = S::Context;
    type Item = S::Item;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut func = self.func;
        self.stream.subscribe_ctx(move |ctx, x| {
            func.call_mut(ctx, x);
            observer(ctx, x);
        })
    }
}

#[cfg(any(test, feature = "slice-deque"))]
pub struct LastN<S, T: Sized> {
    count: usize,
    stream: S,
    data: Rc<RefCell<SliceDeque<T>>>,
}

#[cfg(any(test, feature = "slice-deque"))]
impl<'a, S, T> Stream<'a> for LastN<S, T>
where
    S: Stream<'a, Item = T>,
    T: 'a + Clone + Sized,
{
    type Context = S::Context;
    type Item = [T];

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Context, &Self::Item),
    {
        let data = self.data.clone();
        let count = self.count;
        self.stream.subscribe_ctx(move |ctx, x| {
            let mut queue = data.borrow_mut();
            if queue.len() == count {
                queue.pop_front();
            }
            queue.push_back(x.clone());
            drop(queue); // this is important, in order to avoid multiple mutable borrows
            observer(ctx, &*data.as_ref().borrow());
        })
    }
}

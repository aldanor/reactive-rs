use std::borrow::Borrow;
use std::cell::RefCell;
use std::iter::Iterator;
use std::rc::Rc;

#[cfg(feature = "slice-deque")]
use slice_deque::SliceDeque;

pub trait Stream<'a>: Sized {
    type Context: ?Sized;
    type Item: ?Sized;

    fn subscribe_ctx<O>(self, observer: O)
    where
        O: 'a + FnMut(&Self::Context, &Self::Item);

    fn subscribe<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Item),
    {
        self.subscribe_ctx(move |_ctx, item| observer(item))
    }

    fn broadcast(self) -> ContextBroadcast<'a, Self::Context, Self::Item>
    where
        Self: 'a,
    {
        ContextBroadcast::from_stream(self)
    }

    fn ctx(self) -> Context<Self> {
        Context { stream: self }
    }

    fn with_ctx<T>(self, ctx: T) -> WithContext<Self, T> {
        WithContext { stream: self, ctx }
    }

    fn with_ctx_map<F, T>(self, func: F) -> WithContextMap<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> T,
    {
        WithContextMap { stream: self, func }
    }

    fn map_ctx<F, T>(self, func: F) -> Map<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> T,
    {
        Map { stream: self, func }
    }

    fn map<F, T>(self, func: F) -> Map<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> T,
    {
        Map {
            stream: self,
            func: NoContext(func),
        }
    }

    fn map_both_ctx<F, C, T>(self, func: F) -> MapBoth<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> (C, T),
    {
        MapBoth { stream: self, func }
    }

    fn map_both<F, C, T>(self, func: F) -> MapBoth<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> (C, T),
    {
        MapBoth {
            stream: self,
            func: NoContext(func),
        }
    }

    fn filter_ctx<F>(self, func: F) -> Filter<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> bool,
    {
        Filter { stream: self, func }
    }

    fn filter<F>(self, func: F) -> Filter<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> bool,
    {
        Filter {
            stream: self,
            func: NoContext(func),
        }
    }

    fn filter_map_ctx<F, T>(self, func: F) -> FilterMap<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item) -> Option<T>,
    {
        FilterMap { stream: self, func }
    }

    fn filter_map<F, T>(self, func: F) -> FilterMap<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item) -> Option<T>,
    {
        FilterMap {
            stream: self,
            func: NoContext(func),
        }
    }

    fn fold_ctx<F, T>(self, func: F, init: T) -> Fold<Self, F, T>
    where
        F: 'a + FnMut(&Self::Context, &T, &Self::Item) -> T,
        T: 'a,
    {
        Fold {
            stream: self,
            func,
            value: init,
        }
    }

    fn fold<F, T>(self, func: F, init: T) -> Fold<Self, NoContext<F>, T>
    where
        F: 'a + FnMut(&T, &Self::Item) -> T,
        T: 'a,
    {
        Fold {
            stream: self,
            func: NoContext(func),
            value: init,
        }
    }

    fn inspect_ctx<F, T>(self, func: F) -> Inspect<Self, F>
    where
        F: 'a + FnMut(&Self::Context, &Self::Item),
    {
        Inspect { stream: self, func }
    }

    fn inspect<F, T>(self, func: F) -> Inspect<Self, NoContext<F>>
    where
        F: 'a + FnMut(&Self::Item),
    {
        Inspect {
            stream: self,
            func: NoContext(func),
        }
    }

    #[cfg(feature = "slice-deque")]
    fn last_n(self, count: usize) -> LastN<Self, Self::Item>
    where
        Self::Item: Sized,
    {
        LastN {
            count,
            stream: self,
            data: Rc::new(RefCell::new(SliceDeque::with_capacity(count))),
        }
    }
}

pub trait ContextFn<C, T>
where
    C: ?Sized,
    T: ?Sized,
{
    type Output;

    fn call_mut(&mut self, ctx: &C, item: &T) -> Self::Output;
}

impl<C, T, V> ContextFn<C, T> for FnMut(&C, &T) -> V
where
    C: ?Sized,
    T: ?Sized,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, ctx: &C, item: &T) -> Self::Output {
        self(ctx, item)
    }
}

pub trait ContextFoldFn<C, T, V>
where
    C: ?Sized,
    T: ?Sized,
{
    type Output;

    fn call_mut(&mut self, ctx: &C, acc: &V, item: &T) -> Self::Output;
}

impl<C, T, V> ContextFoldFn<C, T, V> for FnMut(&C, &V, &T) -> V
where
    C: ?Sized,
    T: ?Sized,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, ctx: &C, acc: &V, item: &T) -> Self::Output {
        self(ctx, acc, item)
    }
}

pub struct NoContext<F>(F);

impl<F, C, T, V> ContextFn<C, T> for NoContext<F>
where
    F: FnMut(&T) -> V,
    C: ?Sized,
    T: ?Sized,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, _ctx: &C, item: &T) -> Self::Output {
        (self.0)(item)
    }
}

impl<F, C, T, V> ContextFoldFn<C, T, V> for NoContext<F>
where
    F: FnMut(&V, &T) -> V,
    C: ?Sized,
    T: ?Sized,
{
    type Output = V;

    #[inline(always)]
    fn call_mut(&mut self, _ctx: &C, acc: &Self::Output, item: &T) -> Self::Output {
        (self.0)(acc, item)
    }
}

type Callback<'a, C, T> = Box<'a + FnMut(&C, &T)>;

pub struct ContextBroadcast<'a, C: ?Sized, T: ?Sized> {
    observers: Rc<RefCell<Vec<Callback<'a, C, T>>>>,
}

impl<'a, C, T> ContextBroadcast<'a, C, T>
where
    C: 'a + ?Sized,
    T: 'a + ?Sized,
{
    pub fn new() -> Self {
        Self { observers: Rc::new(RefCell::new(Vec::new())) }
    }

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

    pub fn send<B>(&self, value: B)
    where
        B: Borrow<T>,
        C: Default,
    {
        let ctx = C::default();
        self.send_ctx(&ctx, value);
    }

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

pub type Broadcast<'a, T> = ContextBroadcast<'a, (), T>;

impl<'a, C, T> Default for ContextBroadcast<'a, C, T>
where
    C: 'a + ?Sized,
    T: 'a + ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, C, T> Clone for ContextBroadcast<'a, C, T>
where
    C: 'a + ?Sized,
    T: 'a + ?Sized,
{
    fn clone(&self) -> Self {
        Self { observers: self.observers.clone() }
    }
}

impl<'a, C, T> Stream<'a> for ContextBroadcast<'a, C, T>
where
    C: 'a + ?Sized,
    T: 'a + ?Sized,
{
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

impl<'a, S, T> Stream<'a> for WithContext<S, T>
where
    S: Stream<'a>,
    T: 'a,
{
    type Context = T;
    type Item = S::Item;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let ctx = self.ctx;
        self.stream.subscribe_ctx(
            move |_ctx, x| { observer(&ctx, x); },
        )
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
        self.stream.subscribe_ctx(
            move |ctx, x| { observer(&func(ctx, x), x); },
        )
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
        self.stream.subscribe_ctx(
            move |ctx, _x| { observer(ctx, ctx); },
        )
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
        self.stream.subscribe_ctx(move |ctx, x| {
            observer(ctx, &func.call_mut(ctx, x))
        })
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
        self.stream.subscribe_ctx(
            move |ctx, x| if func.call_mut(ctx, x) {
                observer(ctx, x);
            },
        );
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
        self.stream.subscribe_ctx(
            move |ctx, x| if let Some(x) = func.call_mut(ctx, x) {
                observer(ctx, &x);
            },
        );
    }
}

pub struct Fold<S, F, T> {
    stream: S,
    func: F,
    value: T,
}

impl<'a, S, F, T> Stream<'a> for Fold<S, F, T>
where
    S: Stream<'a>,
    F: 'a + ContextFoldFn<S::Context, S::Item, T, Output = T>,
    T: 'a,
{
    type Context = S::Context;
    type Item = T;

    fn subscribe_ctx<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Context, &Self::Item) + 'a,
    {
        let mut func = self.func;
        let mut value = self.value;
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

#[cfg(feature = "slice-deque")]
pub struct LastN<S, T: Sized> {
    count: usize,
    stream: S,
    data: Rc<RefCell<SliceDeque<T>>>,
}

#[cfg(feature = "slice-deque")]
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

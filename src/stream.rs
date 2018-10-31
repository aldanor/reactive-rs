use std::borrow::Borrow;
use std::cell::RefCell;
use std::iter::Iterator;
use std::rc::Rc;

use slice_deque::SliceDeque;

pub trait Stream<'a> {
    type Item: ?Sized;

    fn subscribe<O>(self, observer: O)
    where
        O: 'a + FnMut(&Self::Item);

    fn fork(self) -> Broadcast<'a, Self::Item>
    where
        Self: 'a + Sized,
    {
        Broadcast::from_stream(self)
    }
}

type Callback<'a, T> = Box<'a + FnMut(&T)>;

pub struct Sink<'a, T: ?Sized> {
    observers: Rc<RefCell<Vec<Callback<'a, T>>>>,
}

impl<'a, T: ?Sized> Sink<'a, T> {
    fn new() -> Self {
        Sink { observers: Rc::new(RefCell::new(Vec::new())) }
    }

    fn push<F>(&self, func: F)
    where
        F: FnMut(&T) + 'a,
    {
        self.observers.borrow_mut().push(Box::new(func));
    }

    pub fn send<B>(&self, value: B)
    where
        B: Borrow<T>,
    {
        let value = value.borrow();
        for observer in self.observers.borrow_mut().iter_mut() {
            observer(value);
        }
    }

    pub fn feed<B, I>(&self, iter: I)
    where
        I: Iterator<Item = B>,
        B: Borrow<T>,
    {
        for value in iter {
            self.send(value);
        }
    }
}

impl<'a, T: ?Sized> Clone for Sink<'a, T> {
    fn clone(&self) -> Self {
        Sink { observers: self.observers.clone() }
    }
}

pub struct Broadcast<'a, T: ?Sized> {
    listeners: Sink<'a, T>,
}

impl<'a, T> Broadcast<'a, T>
where
    T: 'a + ?Sized,
{
    pub fn new() -> Self {
        Broadcast { listeners: Sink::new() }
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<'a, Item = T>,
    {
        let broadcast = Broadcast::new();
        let listeners = broadcast.listeners.clone();
        stream.subscribe(move |x| listeners.send(x));
        broadcast
    }

    pub fn sink(self) -> Sink<'a, T> {
        self.listeners
    }

    pub fn listen(&self) -> Subscription<'a, T> {
        Subscription::new(self.listeners.clone())
    }
}

impl<'a, T> Default for Broadcast<'a, T>
where
    T: 'a + ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct Subscription<'a, T: ?Sized> {
    listeners: Sink<'a, T>,
}

impl<'a, T: ?Sized> Subscription<'a, T> {
    fn new(listeners: Sink<'a, T>) -> Self {
        Subscription { listeners }
    }
}

impl<'a, T> Stream<'a> for Subscription<'a, T> {
    type Item = T;

    fn subscribe<O>(self, observer: O)
    where
        O: FnMut(&Self::Item) + 'a,
    {
        self.listeners.push(observer);
    }
}

pub struct Map<S, M> {
    stream: S,
    func: M,
}

impl<'a, S, M, T> Stream<'a> for Map<S, M>
where
    S: Stream<'a>,
    M: 'a + FnMut(&S::Item) -> T,
{
    type Item = T;

    fn subscribe<O>(self, mut observer: O)
    where
        O: FnMut(&Self::Item) + 'a,
    {
        let mut func = self.func;
        self.stream.subscribe(move |x| observer(&func(x)))
    }
}

pub struct Filter<S, F> {
    stream: S,
    func: F,
}

impl<'a, S, F> Stream<'a> for Filter<S, F>
where
    S: Stream<'a>,
    F: 'a + FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn subscribe<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Item),
    {
        let mut func = self.func;
        self.stream.subscribe(move |x| if func(x) {
            observer(x);
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
    F: 'a + FnMut(&S::Item) -> Option<T>,
{
    type Item = T;

    fn subscribe<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Item),
    {
        let mut func = self.func;
        self.stream.subscribe(move |x| if let Some(x) = func(x) {
            observer(&x);
        });
    }
}

pub struct LastN<S, T: Sized> {
    count: usize,
    stream: S,
    data: Rc<RefCell<SliceDeque<T>>>,
}

impl<'a, S, T> Stream<'a> for LastN<S, T>
where
    S: Stream<'a, Item = T>,
    T: 'a + Clone + Sized,
{
    type Item = [T];

    fn subscribe<O>(self, mut observer: O)
    where
        O: 'a + FnMut(&Self::Item),
    {
        let data = self.data.clone();
        let count = self.count;
        self.stream.subscribe(move |x| {
            let mut queue = data.borrow_mut();
            if queue.len() == count {
                queue.pop_front();
            }
            queue.push_back(x.clone());
            drop(queue); // this is important, in order to avoid multiple mutable borrows
            observer(&*data.as_ref().borrow());
        })
    }
}

pub trait StreamExt<'a>: Stream<'a> + Sized {
    fn map<F, T>(self, func: F) -> Map<Self, F>
    where
        F: 'a + FnMut(&Self::Item) -> T,
    {
        Map { stream: self, func }
    }

    fn filter<F>(self, func: F) -> Filter<Self, F>
    where
        F: 'a + FnMut(&Self::Item) -> bool,
    {
        Filter { stream: self, func }
    }

    fn filter_map<F, T>(self, func: F) -> FilterMap<Self, F>
    where
        F: 'a + FnMut(&Self::Item) -> Option<T>,
    {
        FilterMap { stream: self, func }
    }

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

impl<'a, S: Stream<'a>> StreamExt<'a> for S
where
    S: Stream<'a>,
{
}

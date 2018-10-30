use std::borrow::Borrow;
use std::cell::RefCell;
use std::iter::Iterator;
use std::rc::Rc;

use slice_deque::SliceDeque;

pub trait Stream<'a> {
    type Item: ?Sized;

    fn subscribe<F>(self, f: F) where F: FnMut(&Self::Item) + 'a;

    fn fork(self) -> Broadcast<'a, Self::Item> where Self: 'a + Sized {
        Broadcast::from_stream(self)
    }
}

type Observers<'a, T> = Rc<RefCell<Vec<Box<FnMut(&T) + 'a>>>>;

pub struct Sink<'a, T> where T: ?Sized {
    observers: Observers<'a, T>,
}

impl<'a, T> Sink<'a, T> where T: ?Sized {
    fn new(observers: Observers<'a, T>) -> Self {
        Sink { observers }
    }

    pub fn feed<B>(&self, value: B) where B: Borrow<T> {
        for observer in self.observers.borrow_mut().iter_mut() {
            observer(value.borrow() as &T);
        }
    }

    pub fn feed_iter<B, I>(&self, iter: I) where I: Iterator<Item=B>, B: Borrow<T> {
        for value in iter {
            self.feed(value)
        }
    }
}

pub struct Broadcast<'a, T: ?Sized> {
    observers: Observers<'a, T>,
}

impl<'a, T> Broadcast<'a, T> where T: 'a + ?Sized {
    pub fn new() -> Self {
        Broadcast {
            observers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn from_stream<S>(stream: S) -> Self
        where S: Stream<'a, Item=T>
    {
        let broadcast = Broadcast::new();
        let observers = broadcast.observers.clone();
        stream.subscribe(move |x: &T| {
            for observer in observers.borrow_mut().iter_mut() {
                observer(x);
            }
        });
        broadcast
    }

    pub fn sink(self) -> Sink<'a, T> {
        Sink::new(self.observers)
    }

    pub fn listen(&self) -> Subscription<'a, T> {
        Subscription::new(self.observers.clone())
    }
}

pub struct Subscription<'a, T: ?Sized> {
    observers: Observers<'a, T>,
}

impl<'a, T: ?Sized> Subscription<'a, T> {
    fn new(observers: Observers<'a, T>) -> Self {
        Subscription { observers }
    }
}

impl<'a, T> Stream<'a> for Subscription<'a, T> {
    type Item = T;

    fn subscribe<F>(self, f: F) where F: FnMut(&Self::Item) + 'a {
        self.observers.borrow_mut().push(Box::new(f));
    }
}

pub struct Map<S, M> {
    stream: S,
    func: M,
}

impl<'a, S, M, T> Stream<'a> for Map<S, M>
    where
        S: Stream<'a>,
        M: 'a + FnMut(&S::Item) -> T
{
    type Item = T;

    fn subscribe<F>(self, mut f: F) where F: FnMut(&Self::Item) + 'a {
        let mut func = self.func;
        self.stream.subscribe(move |x| f(&func(x)))
    }
}

pub struct LastN<S, T: Sized> {
    count: usize,
    stream: S,
    data: Rc<RefCell<SliceDeque<T>>>,
}

impl<'a, S, T> Stream<'a> for LastN<S, T>
    where
        S: Stream<'a, Item=T>,
        T: 'a + Clone + Sized
{
    type Item = [T];

    fn subscribe<F>(self, mut f: F) where F: FnMut(&Self::Item) + 'a {
        let data = self.data.clone();
        let count = self.count;
        self.stream.subscribe(move |x| {
            let mut queue = data.borrow_mut();
            if queue.len() == count {
                queue.pop_front();
            }
            queue.push_back(x.clone());
            drop(queue); // this is important, in order to avoid multiple mutable borrows
            f(&*data.as_ref().borrow());
        })
    }
}

pub trait StreamExt<'a>: Stream<'a> + Sized {
    fn map<M, T>(self, func: M) -> Map<Self, M> where M: 'a + FnMut(&Self::Item) -> T {
        Map {
            stream: self,
            func
        }
    }

    fn last_n(self, count: usize) -> LastN<Self, Self::Item> where Self::Item: Sized {
        LastN {
            count,
            stream: self,
            data: Rc::new(RefCell::new(SliceDeque::with_capacity(count)))
        }
    }
}

impl<'a, S: Stream<'a>> StreamExt<'a> for S where S: Stream<'a> {}

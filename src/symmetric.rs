use asymmetric;
use std::iter::Iterator;
use std::cell::UnsafeCell;
use std::default::Default;
use std::ops::DerefMut;
use std::boxed::FnBox;
use context::Context;
use options::Options;

extern crate context;
extern crate libc;

pub static FIRST: i32 = 0;
pub static NONE: i32 = -1;
pub static NEXT: i32 = -2;

pub struct Coors<T> where
    T: Send + 'static {
    current: i32,
    next: i32,
    coroutines: Vec<Coroutine<T>>,
}

impl<T> Coors<T> where
    T: Send {
    pub fn new() -> Coors<T> {
        Coors {
            current: NONE,
            next: NONE,
            coroutines: Vec::new(),
        }
    }

    pub fn set_coroutines(&mut self, coroutines: Vec<Coroutine<T>>) {
       self.coroutines = coroutines;
    }

    pub fn yield_to(&mut self, co: i32, data: T) -> Option<T> {
        self.next = co;
        self.coroutines
            .get(self.current as usize)
            .unwrap()
            .yield_with(data)
    }
    
    pub fn start(&mut self, co: i32, data: T) -> Option<T> {
        self.current = co; 
        let mut arg = Some(data);
        while self.current != NONE {
            arg = self.coroutines
                .get(self.current as usize)
                .unwrap()
                .resume_with(arg.unwrap())
                .unwrap();
            if self.next == NEXT {
                self.current = (self.current + 1) % (self.coroutines.len() as i32);
            } else {
                self.current = self.next;
            }
        }
        arg
    }

    pub fn stop(&mut self, data: T) {
        self.next = -1;
        self.coroutines
            .get(self.current as usize)
            .unwrap()
            .yield_with(data);
    }
}

pub struct Coroutine<T> where
    T: Send + 'static {
    coro: UnsafeCell<Box<asymmetric::CoroutineImpl<T>>>,
}

impl<T> Coroutine<T> where 
    T: Send {
    #[inline]
    pub fn spawn_opts<F>(f: F, opts: Options) -> Coroutine<T> where
        F: FnOnce(&Option<T>) {
        let mut stack = asymmetric::STACK_POOL.with(|pool| unsafe {
            (&mut *pool.get()).take_stack(opts.stack_size)
        });

        let mut coro = Box::new(asymmetric::CoroutineImpl {
            parent: Context::empty(),
            context: Context::empty(),
            stack: None,
            name: opts.name,
            state: asymmetric::State::Created,
            result: None,
        });

        let coro_ref: &mut asymmetric::CoroutineImpl<T> = unsafe {
            let ptr: *mut asymmetric::CoroutineImpl<T> = coro.deref_mut();
            &mut *ptr
        };

        let puller_ref = asymmetric::CoroutineRef {
            coro: coro_ref
        };

        // Coroutine function wrapper
        // Responsible for calling the function and dealing with panicking
        let wrapper = move|| -> ! {
            let ret = unsafe {
                let puller_ref = puller_ref.clone();
                asymmetric::try(|| {
                    let coro_ref: &mut asymmetric::CoroutineImpl<T> = &mut *puller_ref.coro;
                    coro_ref.state = asymmetric::State::Running;
                    f(coro_ref.result.take().unwrap().unwrap().as_ref().unwrap())
                })
            };

            unsafe {
                let coro_ref: &mut asymmetric::CoroutineImpl<T> = &mut *puller_ref.coro;
                coro_ref.state = asymmetric::State::Finished;
            }

            let is_panicked = match ret {
                Ok(..) => false,
                Err(err) => {
                    if let None = err.downcast_ref::<asymmetric::ForceUnwind>() {
                        {
                            let msg = match err.downcast_ref::<&'static str>() {
                                Some(s) => *s,
                                None => match err.downcast_ref::<String>() {
                                    Some(s) => &s[..],
                                    None => "Box<Any>",
                                }
                            };

                            let name = coro_ref.name().unwrap_or("<unnamed>");
                            error!("Coroutine '{}' panicked at '{}'", name, msg);
                        }

                        coro_ref.result = Some(Err(::Error::Panicking(err)));
                        true
                    } else {
                        false
                    }
                }
            };

            loop {
                if is_panicked {
                    coro_ref.result = Some(Err(::Error::Panicked));
                }

                unsafe {
                    coro_ref.yield_back();
                }
            }
        };

        let callback: Box<FnBox()> = Box::new(wrapper);

        coro.context.init_with(asymmetric::coroutine_initialize, 0, Box::into_raw(Box::new(callback)) as *mut libc::c_void, &mut stack);
        coro.stack = Some(stack);

        Coroutine {
            coro: UnsafeCell::new(coro)
        }
    }

    #[inline]
    pub fn spawn<F>(f: F) -> Coroutine<T>
        where F: FnOnce(&Option<T>)
    {
        Coroutine::spawn_opts(f, Default::default())
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        unsafe {
            (&*self.coro.get()).name()
        }
    }

    #[inline]
    pub fn resume(&self) -> ::Result<Option<T>> {
        unsafe {
            (&mut *self.coro.get()).resume()
        }
    }

    #[inline]
    pub fn resume_with(&self, data: T) -> ::Result<Option<T>> {
        unsafe {
            (&mut *self.coro.get()).resume_with(data)
        }
    }
 
    #[inline]
    pub fn yield_with(&self, data: T) -> Option<T> {
        unsafe {
            (&mut *self.coro.get()).yield_with(data)
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn test_symmetric_basic() {
        let mut coors = Coors::new();
        let mut coroutines = Vec::new();

        let coro_1 = Coroutine::spawn(|_| {
            for i in 0..10 {
                coors.yield_to(1, i);
            }
            coors.stop(-53);
        });

        let coro_2 = Coroutine::spawn(|_| {
            for i in 10..20 {
                coors.yield_to(0, i);
            }
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coors.set_coroutines(coroutines);
        coors.start(0, 500000).unwrap();
    }

    #[test]
    fn test_symmetric_basic_args() {
        let mut coors = Coors::new();
        let mut coroutines = Vec::new();

        let coro_1 = Coroutine::spawn(|arg| {
            assert_eq!(arg.unwrap(), "starting");
            assert_eq!(coors.yield_to(1, "yay").unwrap(), "mlem");
            coors.yield_to(1, "back");
        });

        let coro_2 = Coroutine::spawn(|arg| {
            assert_eq!(arg.unwrap(), "yay");
            assert_eq!(coors.yield_to(0, "mlem").unwrap(), "back");
            coors.stop("returning");
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coors.set_coroutines(coroutines);
        assert_eq!(coors.start(0, "starting").unwrap(), "returning");
    }

    #[test]
    fn test_symmetric_complex_args() {
        let mut coors = Coors::new();
        let mut coroutines = Vec::new();
        let coro_1 = Coroutine::spawn(|arg| {
            println!("{}", arg.unwrap());
            for i in 1..4 {
                coors.yield_to(1, i).unwrap();
            }
            coors.stop(1337);
        });

        let coro_2 = Coroutine::spawn(|arg| {
            println!("{}", arg.unwrap());
            for i in 4..8 { 
                coors.yield_to(0, i).unwrap();
            }
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coors.set_coroutines(coroutines);
 
        assert_eq!(coors.start(0, 500000).unwrap(), 1337);
    }

    #[test]
    fn test_symmetric_round_robin() {
        let mut coors = Coors::new();
        let mut coroutines = Vec::new();
        let coro_1 = Coroutine::spawn(|arg| {
            println!("{:?}", arg.unwrap());
            for i in 1..4 {
                coors.yield_to(NEXT, i).unwrap();
            }
            coors.stop(1337);
        });

        let coro_2 = Coroutine::spawn(|arg| {
            println!("{:?}", arg.unwrap());
            for i in 4..8 { 
                coors.yield_to(NEXT, i).unwrap();
            }
        });

        let coro_3 = Coroutine::spawn(|arg| {
            println!("{:?}", arg.unwrap());
            for i in 8..12 { 
                coors.yield_to(NEXT, i).unwrap();
            }
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coroutines.push(coro_3);
        coors.set_coroutines(coroutines);
 
        assert_eq!(coors.start(FIRST, 500000).unwrap(), 1337);
    }

    #[test]
    fn test_symmetric_complex_multiple_args() {
        let mut coors = Coors::new();
        let mut coroutines = Vec::new();
        let coro_1 = Coroutine::spawn(|arg| {
            println!("{:?}", arg.unwrap());
            for i in 1..4 {
                coors.yield_to(NEXT, (i, i+1)).unwrap();
            }
            coors.stop((1337, 23));
        });

        let coro_2 = Coroutine::spawn(|arg| {
            println!("{:?}", arg.unwrap());
            for i in 4..8 { 
                coors.yield_to(NEXT, (i, i+1)).unwrap();
            }
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coors.set_coroutines(coroutines);
 
        assert_eq!(coors.start(FIRST, (500000, 43)).unwrap(), (1337, 23));
    }
}

use asymmetric;
use std::vec::Vec;

struct Coors {
    current: i32,
    next: i32,
    coroutines: Vec<asymmetric::Coroutine<()>>,
}

impl Coors {
    pub fn new() -> Coors {
        Coors {
            current: -1,
            next: -1,
            coroutines: Vec::new(),
        }
    }

    pub fn set_coroutines(&mut self, coroutines: Vec<asymmetric::Coroutine<()>>) {
       self.coroutines = coroutines;
    }

    pub fn yield_to(&mut self, co: i32) {
        self.next = co;
        self.coroutines
            .get(self.current as usize)
            .unwrap()
            .yield_with(());
    }
    
    pub fn start(&mut self, co: i32) {
        self.current = co; 
        while self.current != -1 {
            self.coroutines
                .get(self.current as usize)
                .unwrap()
                .resume()
                .unwrap()
                .unwrap();
            self.current = self.next;
        }
    }

    // TODO: Remove stop?
    pub fn stop(&mut self) {
        self.next = -1;
        self.coroutines
            .get(self.current as usize)
            .unwrap()
            .yield_with(());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use asymmetric;
    use symmetric;
    use std::vec::Vec;

    #[test]
    fn test_my_balls() {
        let coro = asymmetric::Coroutine::spawn(|me| {
            for i in 0..10 {
                me.yield_with(i);
            }
        });

        for (i, j) in coro.zip(0..10) {
            assert_eq!(i.unwrap(), j);
        }
    }

    #[test]
    fn test_symmetric_basic() {
        let mut coors: symmetric::Coors = symmetric::Coors::new();
        let mut coroutines: Vec<asymmetric::Coroutine<()>> = Vec::new();
        let coro_1 = asymmetric::Coroutine::spawn(|me| {
            for i in 0..10 {
                println!("{}", i);
                coors.yield_to(1);
            }
            coors.stop();
        });

        let coro_2 = asymmetric::Coroutine::spawn(|me| {
            for i in 10..20 {
                println!("{}", i);
                coors.yield_to(0);
            }
        });

        coroutines.push(coro_1);
        coroutines.push(coro_2);
        coors.set_coroutines(coroutines);

        coors.start(0);
    }
                

    //#[test]
    //fn test_symmetric_basic() {
    //   let mut coors: symmetric::Coors = symmetric::Coors::new();
    //    let mut coroutines = VecDeque::new();

    //    coroutines.push_back(asymmetric::Coroutine::spawn(|me| {
    //        for i in 0..10 {
    //            println!("{}", i);
    //           let temp = coroutines.pop_front().unwrap();
    //            coroutines.push_back(temp);
    //            coors.yield_to(&temp);
    //        }
    //        coors.stop();
    //    }));

    //    let temp = coroutines.pop_front().unwrap();
    //    coroutines.push_back(temp);
    //    coors.start(&temp);
    //}
}

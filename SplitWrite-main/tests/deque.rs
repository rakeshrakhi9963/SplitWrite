use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use left_right::{
    aliasing::{Aliased, DropBehavior},
    Absorb, ReadHandle,
};

struct Value {
    v: i32,
    r: Rc<ValueRegistry>,
}

impl Value {
    fn new(v: i32, r: Rc<ValueRegistry>) -> Self {
        r.adjust_count(1);
        Self { v, r }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        self.r.adjust_count(-1);
    }
}
struct ValueRegistry {
    num_live_values: Cell<i64>,
}

impl ValueRegistry {
    fn new() -> Self {
        Self {
            num_live_values: Cell::new(0),
        }
    }

    fn adjust_count(&self, delta: i64) {
        let mut live_vals = self.num_live_values.get();
        live_vals += delta;
        assert!(live_vals >= 0);
        self.num_live_values.set(live_vals);
    }

    fn expect(&self, expected_count: i64) {
        assert_eq!(self.num_live_values.get(), expected_count);
    }
}

struct NoDrop;
impl DropBehavior for NoDrop {
    const DO_DROP: bool = false;
}

struct DoDrop;
impl DropBehavior for DoDrop {
    const DO_DROP: bool = true;
}
type Deque = VecDeque<Aliased<Value, NoDrop>>;

enum Op {
    PushBack(Aliased<Value, NoDrop>),
    PopFront,
}

impl Absorb<Op> for Deque {
    fn absorb_first(&mut self, operation: &mut Op, _other: &Self) {
        match operation {
            Op::PushBack(value) => {
                self.push_back(unsafe { value.alias() });
            }
            Op::PopFront => {
                self.pop_front();
            }
        }
    }

    fn absorb_second(&mut self, operation: Op, _other: &Self) {
       
        let with_drop: &mut VecDeque<Aliased<Value, DoDrop>> =
            unsafe { &mut *(self as *mut _ as *mut _) };
        match operation {
            Op::PushBack(value) => {
                with_drop.push_back(unsafe { value.change_drop() });
            }
            Op::PopFront => {
                with_drop.pop_front();
            }
        }
    }

    fn sync_with(&mut self, first: &Self) {
        assert_eq!(self.len(), 0);
        self.extend(first.iter().map(|v| unsafe { v.alias() }));
    }

    fn drop_first(self: Box<Self>) {
        
    }

    fn drop_second(self: Box<Self>) {
        
        let with_drop: Box<VecDeque<Aliased<Value, DoDrop>>> =
            unsafe { Box::from_raw(Box::into_raw(self) as *mut _ as *mut _) };
        drop(with_drop);
    }
}


#[test]
fn deque() {
    let registry = Rc::new(ValueRegistry::new());

    let mkval = |v| Aliased::from(Value::new(v, Rc::clone(&registry)));
    let expect = |r: &ReadHandle<Deque>, expected: &[i32]| {
        let guard = r.enter().unwrap();
        assert!(guard.iter().map(|v| &v.v).eq(expected.iter()));
    };

    let (mut w, r) = left_right::new::<Deque, Op>();
    w.append(Op::PushBack(mkval(1)));
    w.append(Op::PushBack(mkval(2)));
    w.append(Op::PushBack(mkval(3)));
    w.publish();

    registry.expect(3);
    expect(&r, &[1, 2, 3]);

    w.append(Op::PushBack(mkval(4)));
    w.publish();

    registry.expect(4);
    expect(&r, &[1, 2, 3, 4]);

    w.append(Op::PopFront);
    w.append(Op::PopFront);
    w.publish();

    registry.expect(4);
    expect(&r, &[3, 4]);

    w.append(Op::PopFront);
    w.publish();
    
    registry.expect(2);
    expect(&r, &[4]);

    drop(r);
    drop(w);

    registry.expect(0);
}

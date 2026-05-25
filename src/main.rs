use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    ADD,
    SUB,
    MUL,
    DIV,
}

impl Op {
    fn latency(self) -> i32 {
        match self {
            Op::ADD | Op::SUB => 2,
            Op::MUL => 10,
            Op::DIV => 40,
        }
    }

    fn evaluate(self, a: i64, b: i64) -> i64 {
        match self {
            Op::ADD => a + b,
            Op::SUB => a - b,
            Op::MUL => a * b,
            Op::DIV => a / b,
        }
    }
}

#[derive(Clone, Copy)]
struct Inst {
    op: Op,
    dst: usize,
    src1: usize,
    src2: usize,
}

#[derive(Clone, Copy)]
enum RsType {
    ADD,
    MUL,
}

#[derive(Clone)]
struct Rs {
    res_type: RsType,
    idx: usize,
    busy: bool,
    op: Op,
    vj: usize,
    vk: usize,
    qj: usize,
    qk: usize,
    cycles_rem: i32,
    result: i64,
    done: bool,
}

impl Rs {
    fn empty(res_type: RsType, idx: usize) -> Self {
        Rs {
            res_type: res_type,
            idx: idx,
            busy: false,
            op: Op::ADD,
            vj: 0,
            vk: 0,
            qj: 0, // waiting for j value
            qk: 0, // waiting for q value
            cycles_rem: -1,
            result: 0,
            done: false,
        }
    }

    fn ready(&self) -> bool {
        self.busy && self.qj == 0 && self.qk == 0
    }
}

#[derive(Clone)]
struct State {
    rs: Vec<Rs>,
    stat: Vec<usize>, //
    regfile: Vec<i64>,
    op_queue: VecDeque<Inst>, //simulated Op Queue
    cycle: u64,
}

impl State {
    fn new(prog: Vec<Inst>, regs: Vec<i64>) -> Self {
        State {
            rs: vec![
                Rs::empty(RsType::ADD, 0),
                Rs::empty(RsType::ADD, 1),
                Rs::empty(RsType::ADD, 2),
                Rs::empty(RsType::MUL, 0),
                Rs::empty(RsType::MUL, 1),
            ],
            stat: vec![0; 5],
            regfile: regs,
            op_queue: prog.into_iter().collect(),
            cycle: 0,
        }
    }

    fn busy(&self) -> bool {
        !self.op_queue.is_empty() || self.rs.iter().any(|r| r.busy)
    }
}

struct Cdb {
    tag: usize,
    val: i64,
}

// writeback

fn main() {
    return ();
}

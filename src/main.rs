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

#[derive(Clone, Copy, PartialEq)]
enum RsType {
    ADD,
    MUL,
}

impl Op {
    fn rs_type(self) -> RsType {
        match self {
            Op::ADD | Op::SUB => RsType::ADD,
            Op::MUL | Op::DIV => RsType::MUL,
        }
    }
}

// RAT + Regfile
#[derive(Clone, Copy)]
enum RegEntry {
    Ready(i64),
    Pending(usize),
}

#[derive(Clone)]
struct Rs {
    res_type: RsType,
    idx: usize,
    busy: bool,
    op: Option<Op>,
    vj: i64,
    vk: i64,
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
            op: None,
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
    regfile: Vec<RegEntry>, // fused register file + RAT
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
            regfile: regs.into_iter().map(RegEntry::Ready).collect(),
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

fn writeback(curr: &State, mnext: &mut State) -> Option<Cdb> {
    // oldest finished station wins the single CDB
    let winner = curr
        .rs
        .iter()
        .enumerate()
        .filter(|(_, r)| r.busy && r.done)
        .map(|(i, _)| i)
        .next()?;

    let tag = winner + 1;
    let val = curr.rs[winner].result;

    // commit to whichever reg is still waiting on this tag
    for reg in 0..curr.regfile.len() {
        if let RegEntry::Pending(t) = curr.regfile[reg] {
            if t == tag {
                mnext.regfile[reg] = RegEntry::Ready(val);
            }
        }
    }

    // wake up stations waiting on this tag
    for i in 0..curr.rs.len() {
        if curr.rs[i].busy {
            if curr.rs[i].qj == tag {
                mnext.rs[i].vj = val;
                mnext.rs[i].qj = 0;
            }
            if curr.rs[i].qk == tag {
                mnext.rs[i].vk = val;
                mnext.rs[i].qk = 0;
            }
        }
    }

    // free the slot, keep its type/idx
    mnext.rs[winner] = Rs::empty(curr.rs[winner].res_type, curr.rs[winner].idx);

    Some(Cdb { tag, val })
}

fn execute(cur: &State, next: &mut State) {
    for i in 0..5 {
        let r = &cur.rs[i];
        if !r.busy || r.done {
            continue;
        }
        let op = r.op.unwrap();
        if r.cycles_rem < 0 {
            if r.ready() {
                next.rs[i].cycles_rem = op.latency() - 1;
                if next.rs[i].cycles_rem == 0 {
                    next.rs[i].result = op.evaluate(r.vj, r.vk);
                    next.rs[i].done = true;
                }
            }
        }
        else if r.cycles_rem > 0 {
            next.rs[i].cycles_rem = r.cycles_rem - 1;
            if next.rs[i].cycles_rem == 0 {
                next.rs[i].result = op.evaluate(r.vj, r.vk);
                next.rs[i].done = true;
            }
        }
    }
}

fn issue(cur: &State, next: &mut State) -> Option<usize> {
    let inst = *cur.op_queue.front()?;

    // find a free station of the matching type
    let want = inst.op.rs_type();
    let slot = cur
        .rs
        .iter()
        .position(|r| !r.busy && r.res_type == want)?;

    next.op_queue.pop_front();

    let tag = slot + 1;
    let mut r = Rs::empty(cur.rs[slot].res_type, cur.rs[slot].idx);
    r.busy = true;
    r.op = Some(inst.op);

    // rename src1 -> vj/qj
    match cur.regfile[inst.src1] {
        RegEntry::Ready(v) => r.vj = v,
        RegEntry::Pending(t) => r.qj = t,
    }
    // rename src2 -> vk/qk
    match cur.regfile[inst.src2] {
        RegEntry::Ready(v) => r.vk = v,
        RegEntry::Pending(t) => r.qk = t,
    }

    next.rs[slot] = r;
    next.regfile[inst.dst] = RegEntry::Pending(tag); // this station now owns dst

    Some(slot)
}

fn tick(cur: &State) -> (State, Option<Cdb>, Option<usize>) {
    let mut next = cur.clone();
    next.cycle = cur.cycle + 1;
    let cdb = writeback(cur, &mut next);
    execute(cur, &mut next);
    let issued = issue(cur, &mut next);
    (next, cdb, issued)
}


fn dump(s: &State, cdb: &Option<Cdb>, issued: &Option<usize>) {
    println!("================ cycle {} ================", s.cycle);
    if let Some(i) = issued {
        println!("  ISSUE   -> RS{}", i + 1);
    }
    if let Some(c) = cdb {
        println!("  CDB     <- tag {}  value {}", c.tag, c.val);
    }
    println!("  RS   type op   Qj Qk   Vj      Vk      rem  done");
    for (i, r) in s.rs.iter().enumerate() {
        if r.busy {
            let ty = match r.res_type {
                RsType::ADD => "ADD",
                RsType::MUL => "MUL",
            };
            let op = match r.op {
                Some(o) => format!("{:?}", o),
                None => "-".to_string(),
            };
            println!(
                "  {:<4} {:<4} {:<4} {:<2} {:<2}   {:<7} {:<7} {:<4} {}",
                format!("RS{}", i + 1),
                ty,
                op,
                r.qj,
                r.qk,
                r.vj,
                r.vk,
                r.cycles_rem,
                r.done as u8,
            );
        }
    }
    let rat: Vec<String> = s
        .regfile
        .iter()
        .enumerate()
        .filter_map(|(reg, e)| match e {
            RegEntry::Pending(t) => Some(format!("R{}=tag{}", reg, t)),
            RegEntry::Ready(_) => None,
        })
        .collect();
    println!("  RAT: [{}]", rat.join(", "));
}

fn main() {
    // R[i] = i * 10
    let nregs = 12;
    let mut regs = vec![0i64; nregs];
    for i in 0..nregs {
        regs[i] = (i as i64) * 10;
    }

    use Op::*;
    let prog = vec![
        Inst { op: MUL, dst: 1, src1: 2, src2: 3 }, // R1 = R2 * R3 (slow, MUL station)
        Inst { op: ADD, dst: 4, src1: 1, src2: 5 }, // R4 = R1 + R5 (waits on R1)
        Inst { op: ADD, dst: 6, src1: 7, src2: 8 }, // R6 = R7 + R8 (independent, ADD station)
        Inst { op: SUB, dst: 9, src1: 6, src2: 2 }, // R9 = R6 - R2 (waits on R6)
    ];

    let mut state = State::new(prog, regs);

    let mut guard = 0;
    while state.busy() && guard < 200 {
        let (next, cdb, issued) = tick(&state);
        state = next;
        dump(&state, &cdb, &issued);
        guard += 1;
    }

    println!("\n================ final regfile ================");
    for (i, e) in state.regfile.iter().enumerate() {
        match e {
            RegEntry::Ready(v) => println!("  R{:<2} = {}", i, v),
            RegEntry::Pending(t) => println!("  R{:<2} = <pending tag{}>", i, t),
        }
    }
    // Expected: R1=600, R4=650, R6=150, R9=130
}

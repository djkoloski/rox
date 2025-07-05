use rox::{Chunk, Op, Value, VirtualMachine};

fn main() {
    let mut chunk = Chunk::new();

    let a = chunk.add_constant(Value::Float(1.2));
    let b = chunk.add_constant(Value::Float(3.4));
    let c = chunk.add_constant(Value::Float(5.6));

    chunk.encode_constant(a, 123);
    chunk.encode_constant(b, 123);
    chunk.encode(Op::Add, 123);
    chunk.encode_constant(c, 123);
    chunk.encode(Op::Divide, 123);
    chunk.encode(Op::Negate, 123);
    chunk.encode(Op::Return, 123);

    let mut vm = VirtualMachine::new(&chunk);
    vm.interpret().unwrap();
}

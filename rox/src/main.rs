use rox::{Chunk, Op, Value, VirtualMachine};

fn main() {
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(Value::Float(1.2));
    chunk.encode_constant(constant, 123);

    chunk.encode(Op::Return, 123);

    chunk.disassemble("test chunk");

    let mut vm = VirtualMachine::new(&chunk);
    vm.interpret().unwrap();
}

mod vm;

use vm::VM;

fn main() {
    let bytecode = [0x60, 0x0A, 0x60, 0x05, 0x01, 0x00];
    let mut vm = VM::new();
    match vm.execute(&bytecode) {
        Ok(_)  => println!("ok"),
        Err(e) => println!("err: {}", e),
    }
}

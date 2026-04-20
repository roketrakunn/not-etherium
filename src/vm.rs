use std::{collections::HashMap, result};

struct VM {
    stack:   Vec<[u8; 32]>,
    pc:      u64,
    gas:     u64,
    memory:  Vec<u8>,
    storage: HashMap<[u8; 32], [u8; 32]>,
}

impl VM {

    pub fn new() -> Self {
        VM {
            stack:   Vec::new(),
            pc:      0,
            gas:     0,
            memory:  Vec::new(),
            storage: HashMap::new(),
        }
    }

    pub fn execute(&mut self, bytecode: &[u8]) -> Result<(), String> {
        while (self.pc as usize) < bytecode.len() {
            let opcode = bytecode[self.pc as usize];
            self.pc += 1;

            match opcode {
                // STOP
                0x00 => return Ok(()),

                // ADD: pop two, push sum (wrapping 256-bit)
                0x01 => {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(add_u256(a, b));
                }

                0x02 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(mul_u256(a ,b));
                }

                // PUSH1: read next byte, push it as a 32-byte value
                0x60 => {
                    if (self.pc as usize) >= bytecode.len() {
                        return Err("PUSH1: missing operand".into());
                    }
                    let byte = bytecode[self.pc as usize];
                    self.pc += 1;
                    let mut val = [0u8; 32];
                    val[31] = byte;
                    self.push(val);
                }

                // POP: discard top of stack
                0x50 => { self.pop()?; }

                unknown => return Err(format!("unknown opcode: 0x{:02x}", unknown)),
            }
        }

        Ok(())
    }

    fn push(&mut self, val: [u8; 32]) {
        self.stack.push(val);
    }

    fn pop(&mut self) -> Result<[u8; 32], String> {
        self.stack.pop().ok_or_else(|| "stack underflow".into())
    }
}

// wrapping big-endian addition for 256-bit values
fn add_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut carry: u16 = 0;
    for i in (0..32).rev() {
        let sum = a[i] as u16 + b[i] as u16 + carry;
        result[i] = sum as u8;
        carry = sum >> 8;
    }
    result
}

fn mul_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] { 
    let mut result =[0u8; 32];

    let  a_hi  = u128::from_be_bytes(a[0..16].try_into().unwrap());
    let a_lo  = u128::from_be_bytes(a[16..32].try_into().unwrap());

    let b_hi  = u128::from_be_bytes(b[0..16].try_into().unwrap());
    let b_lo  = u128::from_be_bytes(b[16..32].try_into().unwrap());

    let ll = a_lo * b_lo;
    let hl = (a_hi * b_lo) << 128; 
    let lh = (b_hi * a_lo) << 128;
    
    result
}

use std::collections::HashMap;

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

                // MUL (same as add but uses mul_256)
                0x02 => { 
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(mul_u256(a ,b));
                }

                0x03 =>  {
                    let a = self.pop()?;
                    let b = self.pop()?;
                    self.push(sub_u256(a,b));
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

fn sub_u256(a: [u8; 32], b :[u8; 32]) -> [u8; 32] { 
    let mut result = [0u8; 32]; 

    let mut borrow : i16 = 0 ; 

    for i in (0..32).rev() { 
        let diff = a[i] as i16 - b[i] as i16 - borrow;
        result[i] = diff as u8;

        borrow = diff >>  8;
    }

    result
}

fn mul_u256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let a_hi = u128::from_be_bytes(a[0..16].try_into().unwrap());
    let a_lo = u128::from_be_bytes(a[16..32].try_into().unwrap());
    let b_hi = u128::from_be_bytes(b[0..16].try_into().unwrap());
    let b_lo = u128::from_be_bytes(b[16..32].try_into().unwrap());

    // a_lo * b_lo produces up to 256 bits — split into hi/lo using 64-bit 
    let (a0, a1) = (a_lo >> 64, a_lo & u64::MAX as u128);
    let (b0, b1) = (b_lo >> 64, b_lo & u64::MAX as u128);
    let p0 = a1 * b1;
    let p1 = a1 * b0;
    let p2 = a0 * b1;
    let p3 = a0 * b0;
    let mid = (p0 >> 64) + (p1 & u64::MAX as u128) + (p2 & u64::MAX as u128);
    let ll_lo = (mid << 64) | (p0 & u64::MAX as u128);
    let ll_hi = p3 + (p1 >> 64) + (p2 >> 64) + (mid >> 64);

    // hl and lh shift left 128, so they go directly into result_hi
    let result_hi = ll_hi
        .wrapping_add(a_hi.wrapping_mul(b_lo))
        .wrapping_add(a_lo.wrapping_mul(b_hi));

    let mut result = [0u8; 32];
    result[0..16].copy_from_slice(&result_hi.to_be_bytes());
    result[16..32].copy_from_slice(&ll_lo.to_be_bytes());
    result
}
